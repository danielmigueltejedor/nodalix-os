//! Displays: resolution, refresh rate, scale, orientation, which screen is the main one and where
//! the screens sit (drag them), through mutter's `DisplayConfig`. A change is applied for a few
//! seconds first and kept only if the person confirms it, so a setting the screen cannot show
//! never strands them. All the rules about what is a valid layout are in `ajustes-core`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::displays::{Config, Logical, Mode, Monitor};

use super::gs;
use super::{bus, column, scroller};
use crate::i18n::tr;
use crate::rows;

const MUTTER: &str = "org.gnome.Mutter.DisplayConfig";
const MUTTER_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const COLOR: &str = "org.gnome.settings-daemon.plugins.color";
const CONFIRM_SECONDS: u32 = 20;
const ORIENTATIONS: [&str; 4] = [
    "Landscape",
    "Portrait (rotated left)",
    "Landscape (flipped)",
    "Portrait (rotated right)",
];

/// A value of an `a{sv}` dictionary.
fn lookup(dict: &glib::Variant, key: &str) -> Option<glib::Variant> {
    glib::VariantDict::new(Some(dict)).lookup_value(key, None)
}

fn text(v: &glib::Variant, i: usize) -> String {
    v.try_child_value(i)
        .and_then(|v| v.str().map(str::to_owned))
        .unwrap_or_default()
}

/// The configuration in mutter's `GetCurrentState` reply
/// `(u, a((ssss)a(siiddada{sv})a{sv}), a(iiduba(ssss)a{sv}), a{sv})`. What does not parse is left
/// out, so a changed API shows fewer screens, not a crash.
pub fn parse_state(state: &glib::Variant) -> Option<Config> {
    let serial = state.try_child_value(0)?.get::<u32>()?;
    let mut monitors = Vec::new();
    for monitor in state.try_child_value(1)?.iter() {
        let ids = monitor.try_child_value(0)?;
        let connector = text(&ids, 0);
        let name = monitor
            .try_child_value(2)
            .and_then(|p| lookup(&p, "display-name"))
            .and_then(|v| v.str().map(str::to_owned))
            .unwrap_or_else(|| {
                format!("{} {}", text(&ids, 1), text(&ids, 2))
                    .trim()
                    .to_owned()
            });
        let mut modes = Vec::new();
        for m in monitor.try_child_value(1)?.iter() {
            let props = m.try_child_value(6);
            let flag = |key: &str| {
                props
                    .as_ref()
                    .and_then(|p| lookup(p, key))
                    .and_then(|v| v.get::<bool>())
                    .unwrap_or(false)
            };
            let mode = Mode {
                id: text(&m, 0),
                width: m.try_child_value(1)?.get::<i32>()?,
                height: m.try_child_value(2)?.get::<i32>()?,
                refresh: m.try_child_value(3)?.get::<f64>()?,
                scales: m.try_child_value(5)?.get::<Vec<f64>>().unwrap_or_default(),
                preferred: flag("is-preferred"),
                variable: props
                    .as_ref()
                    .and_then(|p| lookup(p, "refresh-rate-mode"))
                    .and_then(|v| v.str().map(|s| s == "variable"))
                    .unwrap_or(false),
            };
            // The mode in use is remembered by the screen that shows it, below.
            if flag("is-current") {
                modes.push((mode, true));
            } else {
                modes.push((mode, false));
            }
        }
        monitors.push((
            Monitor {
                connector,
                name,
                modes: modes.iter().map(|(m, _)| m.clone()).collect(),
            },
            modes.into_iter().find(|(_, c)| *c).map(|(m, _)| m.id),
        ));
    }
    let mut logicals = Vec::new();
    for l in state.try_child_value(2)?.iter() {
        let mut shown = Vec::new();
        for ids in l.try_child_value(5)?.iter() {
            let connector = text(&ids, 0);
            let Some((_, Some(mode))) = monitors.iter().find(|(m, _)| m.connector == connector)
            else {
                continue;
            };
            shown.push((connector, mode.clone()));
        }
        if shown.is_empty() {
            continue;
        }
        logicals.push(Logical {
            x: l.try_child_value(0)?.get::<i32>()?,
            y: l.try_child_value(1)?.get::<i32>()?,
            scale: l.try_child_value(2)?.get::<f64>()?,
            transform: l.try_child_value(3)?.get::<u32>()?,
            primary: l.try_child_value(4)?.get::<bool>()?,
            monitors: shown,
        });
    }
    Some(Config {
        serial,
        monitors: monitors.into_iter().map(|(m, _)| m).collect(),
        logicals,
    })
}

/// What `ApplyMonitorsConfig` takes: `(u serial, u method, a(iiduba(ssa{sv})), a{sv})`.
fn apply_arguments(config: &Config, serial: u32, method: u32) -> glib::Variant {
    let empty = || HashMap::<String, glib::Variant>::new().to_variant();
    let logicals = config.logicals.iter().map(|l| {
        let monitors = l.monitors.iter().map(|(connector, mode)| {
            glib::Variant::tuple_from_iter([connector.to_variant(), mode.to_variant(), empty()])
        });
        glib::Variant::tuple_from_iter([
            l.x.to_variant(),
            l.y.to_variant(),
            l.scale.to_variant(),
            l.transform.to_variant(),
            l.primary.to_variant(),
            glib::Variant::array_from_iter_with_type(
                glib::VariantTy::new("(ssa{sv})").expect("a valid type"),
                monitors,
            ),
        ])
    });
    glib::Variant::tuple_from_iter([
        serial.to_variant(),
        method.to_variant(),
        glib::Variant::array_from_iter_with_type(
            glib::VariantTy::new("(iiduba(ssa{sv}))").expect("a valid type"),
            logicals,
        ),
        empty(),
    ])
}

/// `60 Hz`, `59.94 Hz`: the rate without a decimal part when it has none.
const MUTTER_SETTINGS: &str = "org.gnome.mutter";
const VRR_FEATURE: &str = "variable-refresh-rate";

/// Whether the desktop offers adaptive sync (`None` when it has no such option).
fn experimental_vrr() -> Option<bool> {
    let g = super::gs::open(MUTTER_SETTINGS).filter(|g| g.has("experimental-features"))?;
    Some(
        g.settings()
            .strv("experimental-features")
            .iter()
            .any(|f| f == VRR_FEATURE),
    )
}

/// Adds or removes adaptive sync among the desktop's experimental features, leaving the rest.
fn set_experimental_vrr(on: bool) {
    let Some(g) = super::gs::open(MUTTER_SETTINGS).filter(|g| g.has("experimental-features"))
    else {
        return;
    };
    let mut features: Vec<String> = g
        .settings()
        .strv("experimental-features")
        .iter()
        .map(|f| f.to_string())
        .filter(|f| f != VRR_FEATURE)
        .collect();
    if on {
        features.push(VRR_FEATURE.to_owned());
    }
    let features: Vec<&str> = features.iter().map(String::as_str).collect();
    let _ = g.settings().set_strv("experimental-features", features);
}

fn rate_text(refresh: f64) -> String {
    let rounded = (refresh * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.005 {
        format!("{} Hz", rounded.round() as i64)
    } else {
        format!("{rounded} Hz")
    }
}

struct Page {
    widget: gtk::ScrolledWindow,
    toasts: ndl::ToastOverlay,
    /// What mutter reported last, and what is being edited.
    original: RefCell<Option<Config>>,
    edited: RefCell<Option<Config>>,
    selected: Cell<usize>,
    loading: Cell<bool>,
    busy: Cell<bool>,
    canvas: gtk::DrawingArea,
    drag: RefCell<Option<Drag>>,
    screen_group: ndl::PreferencesGroup,
    screen: ndl::ComboRow,
    resolution: ndl::ComboRow,
    refresh: ndl::ComboRow,
    vrr: ndl::SwitchRow,
    vrr_feature: ndl::SwitchRow,
    scale: ndl::ComboRow,
    orientation: ndl::ComboRow,
    primary: ndl::SwitchRow,
    off: gtk::Button,
    off_row: ndl::ActionRow,
    disabled_group: ndl::PreferencesGroup,
    disabled_rows: RefCell<Vec<gtk::Widget>>,
    arrange_group: ndl::PreferencesGroup,
    apply: gtk::Button,
    revert: gtk::Button,
    message: ndl::ActionRow,
}

struct Drag {
    index: usize,
    start: (i32, i32),
    offset: (f64, f64),
}

/// How the canvas maps desktop pixels to widget pixels: `(scale, origin x, origin y)`.
fn view(config: &Config, width: f64, height: f64) -> (f64, f64, f64) {
    let pad = 14.0;
    let (mut right, mut bottom) = (1, 1);
    for i in 0..config.logicals.len() {
        let (w, h) = config.size(i);
        right = right.max(config.logicals[i].x + w);
        bottom = bottom.max(config.logicals[i].y + h);
    }
    // Room around the layout, so a screen can be dragged out to any side.
    let k = ((width - 2.0 * pad) / f64::from(right) / 1.6)
        .min((height - 2.0 * pad) / f64::from(bottom) / 1.6)
        .max(0.01);
    let ox = (width - f64::from(right) * k) / 2.0;
    let oy = (height - f64::from(bottom) * k) / 2.0;
    (k, ox, oy)
}

impl Page {
    fn config(&self) -> Option<Config> {
        self.edited.borrow().clone()
    }

    fn dirty(&self) -> bool {
        *self.original.borrow() != *self.edited.borrow()
    }

    fn say(&self, text: &str) {
        self.toasts.add_toast(ndl::Toast::new(text));
    }

    /// Edits the configuration with `f`, keeping the result only if it is still valid.
    fn edit(self: &Rc<Self>, f: impl FnOnce(&mut Config, usize)) {
        if self.loading.get() {
            return;
        }
        let Some(mut config) = self.config() else {
            return;
        };
        let index = self
            .selected
            .get()
            .min(config.logicals.len().saturating_sub(1));
        let before = config.clone();
        f(&mut config, index);
        if config != before && config.is_valid() {
            *self.edited.borrow_mut() = Some(config);
        }
        self.render();
    }

    /// Sets every widget from the configuration being edited.
    fn render(self: &Rc<Self>) {
        self.loading.set(true);
        let Some(config) = self.config() else {
            self.loading.set(false);
            return;
        };
        let count = config.logicals.len();
        let index = self.selected.get().min(count.saturating_sub(1));
        self.selected.set(index);

        self.arrange_group.set_visible(count > 1);
        self.screen.set_visible(count > 1);
        let names: Vec<String> = (0..count)
            .map(|i| {
                let name = config.logicals[i]
                    .monitors
                    .first()
                    .and_then(|(c, _)| config.monitor(c))
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                format!("{} · {}", i + 1, name)
            })
            .collect();
        let labels: Vec<&str> = names.iter().map(String::as_str).collect();
        self.screen.set_model(Some(&gtk::StringList::new(&labels)));
        self.screen.set_selected(index as u32);

        let sizes = config.resolutions(index);
        let size_labels: Vec<String> = sizes.iter().map(|(w, h)| format!("{w} × {h}")).collect();
        let size_refs: Vec<&str> = size_labels.iter().map(String::as_str).collect();
        self.resolution
            .set_model(Some(&gtk::StringList::new(&size_refs)));
        if let Some(mode) = config.mode_of(index) {
            if let Some(pos) = sizes.iter().position(|s| *s == (mode.width, mode.height)) {
                self.resolution.set_selected(pos as u32);
            }
            let rates = config.refresh_rates(index, (mode.width, mode.height));
            let rate_labels: Vec<String> = rates.iter().map(|m| rate_text(m.refresh)).collect();
            let rate_refs: Vec<&str> = rate_labels.iter().map(String::as_str).collect();
            self.refresh
                .set_model(Some(&gtk::StringList::new(&rate_refs)));
            if let Some(pos) = rates.iter().position(|m| m.id == mode.id) {
                self.refresh.set_selected(pos as u32);
            }
            // With one rate there is nothing to choose.
            self.refresh.set_sensitive(rates.len() > 1);
        }
        let feature = experimental_vrr();
        self.vrr_feature.set_visible(feature.is_some());
        self.vrr_feature.set_active(feature == Some(true));
        let available = config.vrr_available(index);
        self.vrr.set_active(config.vrr_on(index));
        self.vrr.set_sensitive(available);
        self.vrr.set_subtitle(tr(if available {
            "The screen follows the frame rate of games and videos (FreeSync, G-Sync Compatible)"
        } else if feature == Some(false) {
            "Turn on \"Offer adaptive refresh rate\" below first"
        } else {
            "This screen does not offer it"
        }));
        let scales = config.scales(index);
        let scale_labels: Vec<String> = scales
            .iter()
            .map(|s| format!("{} %", (s * 100.0).round() as i64))
            .collect();
        let scale_refs: Vec<&str> = scale_labels.iter().map(String::as_str).collect();
        self.scale
            .set_model(Some(&gtk::StringList::new(&scale_refs)));
        if let Some(l) = config.logicals.get(index) {
            if let Some(pos) = scales.iter().position(|s| (s - l.scale).abs() < 0.001) {
                self.scale.set_selected(pos as u32);
            }
            self.scale.set_sensitive(scales.len() > 1);
            if l.transform < 4 {
                self.orientation.set_selected(l.transform);
            }
            self.primary.set_active(l.primary);
            self.primary.set_visible(count > 1);
            // The main screen cannot be switched off from here; make another the main one first.
            self.primary.set_sensitive(!l.primary);
        }
        self.off_row.set_visible(count > 1);
        self.off.set_sensitive(count > 1);

        for row in self.disabled_rows.borrow_mut().drain(..) {
            self.disabled_group.remove(&row);
        }
        let disabled = config.disabled();
        self.disabled_group.set_visible(!disabled.is_empty());
        for monitor in disabled {
            let row = ndl::ActionRow::builder().title(&monitor.name).build();
            row.set_subtitle(&monitor.connector);
            row.add_prefix(&gtk::Image::from_icon_name("video-display-symbolic"));
            let button = gtk::Button::with_label(tr("Turn on"));
            button.set_valign(gtk::Align::Center);
            let (page, connector) = (self.clone(), monitor.connector.clone());
            button.connect_clicked(move |_| {
                let connector = connector.clone();
                page.edit(move |c, _| {
                    c.enable(&connector);
                });
            });
            row.add_suffix(&button);
            self.disabled_group.add(&row);
            self.disabled_rows.borrow_mut().push(row.upcast());
        }

        let dirty = self.dirty() && !self.busy.get();
        self.apply.set_sensitive(dirty);
        self.revert.set_sensitive(dirty);
        self.canvas.queue_draw();
        self.loading.set(false);
    }

    /// Reads mutter's state again; the screens may have changed by themselves.
    fn load(self: &Rc<Self>) {
        let page = self.clone();
        glib::spawn_future_local(async move {
            let state = async {
                let connection = bus::session().await?;
                let reply = bus::call(
                    &connection,
                    MUTTER,
                    MUTTER_PATH,
                    MUTTER,
                    "GetCurrentState",
                    None,
                )
                .await
                .ok()?;
                parse_state(&reply)
            }
            .await;
            match state {
                Some(config) if !config.logicals.is_empty() => {
                    page.message.set_visible(false);
                    page.screen_group.set_visible(true);
                    *page.original.borrow_mut() = Some(config.clone());
                    *page.edited.borrow_mut() = Some(config);
                    page.render();
                }
                _ => {
                    page.screen_group.set_visible(false);
                    page.arrange_group.set_visible(false);
                    page.message.set_visible(true);
                }
            }
        });
    }

    /// Applies `config` as `method` (1 for a trial, 2 to keep) with the serial mutter gave.
    async fn send(config: &Config, method: u32) -> Result<(), String> {
        let connection = bus::session()
            .await
            .ok_or_else(|| tr("The screens could not be read").to_owned())?;
        let args = apply_arguments(config, config.serial, method);
        bus::call(
            &connection,
            MUTTER,
            MUTTER_PATH,
            MUTTER,
            "ApplyMonitorsConfig",
            Some(&args),
        )
        .await
        .map(|_| ())
        .map_err(|mut e| {
            gio::DBusError::strip_remote_error(&mut e);
            e.message().to_owned()
        })
    }

    /// The current state from mutter, for the serial a following call must carry.
    async fn current() -> Option<Config> {
        let connection = bus::session().await?;
        let reply = bus::call(
            &connection,
            MUTTER,
            MUTTER_PATH,
            MUTTER,
            "GetCurrentState",
            None,
        )
        .await
        .ok()?;
        parse_state(&reply)
    }

    fn apply_edited(self: &Rc<Self>) {
        let (Some(edited), Some(original)) = (self.config(), self.original.borrow().clone()) else {
            return;
        };
        if !edited.is_valid() || self.busy.replace(true) {
            return;
        }
        self.render();
        let page = self.clone();
        glib::spawn_future_local(async move {
            match Self::send(&edited, 1).await {
                Err(error) => {
                    page.say(&error);
                    page.busy.set(false);
                    page.render();
                }
                Ok(()) => {
                    let window = page.widget.root().and_downcast::<gtk::Window>();
                    let keep = confirm(window.as_ref()).await;
                    // A new serial after the change; the next call must carry it.
                    let fresh = Self::current().await;
                    let serial = fresh.as_ref().map_or(edited.serial, |c| c.serial);
                    let target = if keep { &edited } else { &original };
                    let outcome = Self::send(
                        &Config {
                            serial,
                            ..target.clone()
                        },
                        if keep { 2 } else { 1 },
                    )
                    .await;
                    if let Err(error) = outcome {
                        page.say(&error);
                    }
                    page.busy.set(false);
                    page.load();
                }
            }
        });
    }
}

/// The "keep these settings?" window: `true` to keep, `false` to go back (also when the time
/// runs out or the window is closed).
async fn confirm(parent: Option<&gtk::Window>) -> bool {
    let (sender, receiver) = futures_channel_oneshot();
    let window = gtk::Window::builder()
        .modal(true)
        .resizable(false)
        .title(tr("Keep these display settings?"))
        .build();
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }
    let remaining = Rc::new(Cell::new(CONFIRM_SECONDS));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    body.set_margin_start(24);
    body.set_margin_end(24);
    let title = gtk::Label::new(Some(tr("Keep these display settings?")));
    title.add_css_class("title-3");
    let label = gtk::Label::new(None);
    label.set_wrap(true);
    label.set_max_width_chars(40);
    let show = {
        let label = label.clone();
        move |n: u32| {
            label.set_text(
                &tr("The previous settings come back in %d seconds").replace("%d", &n.to_string()),
            );
        }
    };
    show(CONFIRM_SECONDS);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let revert = gtk::Button::with_label(tr("Revert settings"));
    let keep = gtk::Button::with_label(tr("Keep changes"));
    keep.add_css_class("suggested-action");
    buttons.append(&revert);
    buttons.append(&keep);
    body.append(&title);
    body.append(&label);
    body.append(&buttons);
    window.set_child(Some(&body));

    let sender = Rc::new(RefCell::new(Some(sender)));
    let finish = {
        let (sender, window) = (sender.clone(), window.clone());
        move |keep: bool| {
            if let Some(s) = sender.borrow_mut().take() {
                let _ = s.send(keep);
            }
            window.close();
        }
    };
    {
        let finish = finish.clone();
        keep.connect_clicked(move |_| finish(true));
    }
    {
        let finish = finish.clone();
        revert.connect_clicked(move |_| finish(false));
    }
    {
        let finish = finish.clone();
        window.connect_close_request(move |_| {
            finish(false);
            glib::Propagation::Proceed
        });
    }
    {
        let (remaining, finish, weak) = (remaining.clone(), finish.clone(), window.downgrade());
        glib::timeout_add_seconds_local(1, move || {
            if weak.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            let left = remaining.get().saturating_sub(1);
            remaining.set(left);
            show(left);
            if left == 0 {
                finish(false);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
    window.present();
    receiver.await.unwrap_or(false)
}

/// A one-shot channel; `glib` has no dependency on a futures crate here.
fn futures_channel_oneshot() -> (OneshotSender, OneshotReceiver) {
    let shared = Rc::new(RefCell::new((None::<bool>, None::<std::task::Waker>)));
    (OneshotSender(shared.clone()), OneshotReceiver(shared))
}

struct OneshotSender(Rc<RefCell<(Option<bool>, Option<std::task::Waker>)>>);
struct OneshotReceiver(Rc<RefCell<(Option<bool>, Option<std::task::Waker>)>>);

impl OneshotSender {
    fn send(self, value: bool) -> Result<(), bool> {
        let mut state = self.0.borrow_mut();
        state.0 = Some(value);
        if let Some(waker) = state.1.take() {
            waker.wake();
        }
        Ok(())
    }
}

impl std::future::Future for OneshotReceiver {
    type Output = Option<bool>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        match state.0.take() {
            Some(v) => std::task::Poll::Ready(Some(v)),
            None => {
                state.1 = Some(cx.waker().clone());
                std::task::Poll::Pending
            }
        }
    }
}

fn draw(
    page: &Rc<Page>,
    area: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
) {
    let Some(config) = page.config() else { return };
    let (w, h) = (f64::from(width), f64::from(height));
    let (k, ox, oy) = view(&config, w, h);
    let color = area.color();
    let dragging = page.drag.borrow();
    for i in 0..config.logicals.len() {
        let l = &config.logicals[i];
        let (sw, sh) = config.size(i);
        let (mut x, mut y) = (f64::from(l.x), f64::from(l.y));
        if let Some(d) = dragging.as_ref().filter(|d| d.index == i) {
            x = f64::from(d.start.0) + d.offset.0 / k;
            y = f64::from(d.start.1) + d.offset.1 / k;
        }
        let (rx, ry, rw, rh) = (ox + x * k, oy + y * k, f64::from(sw) * k, f64::from(sh) * k);
        let selected = i == page.selected.get();
        let radius = 8.0;
        cr.new_sub_path();
        cr.arc(
            rx + rw - radius,
            ry + radius,
            radius,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );
        cr.arc(
            rx + rw - radius,
            ry + rh - radius,
            radius,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        cr.arc(
            rx + radius,
            ry + rh - radius,
            radius,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cr.arc(
            rx + radius,
            ry + radius,
            radius,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        );
        cr.close_path();
        let (r, g, b) = (
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
        );
        cr.set_source_rgba(r, g, b, if selected { 0.16 } else { 0.07 });
        let _ = cr.fill_preserve();
        cr.set_source_rgba(r, g, b, if selected { 0.95 } else { 0.35 });
        cr.set_line_width(if selected { 2.5 } else { 1.2 });
        let _ = cr.stroke();
        cr.set_source_rgba(r, g, b, 0.9);
        cr.set_font_size((rh * 0.38).clamp(12.0, 34.0));
        let label = if l.primary {
            format!("{} ★", i + 1)
        } else {
            (i + 1).to_string()
        };
        if let Ok(extents) = cr.text_extents(&label) {
            cr.move_to(
                rx + (rw - extents.width()) / 2.0 - extents.x_bearing(),
                ry + (rh - extents.height()) / 2.0 - extents.y_bearing(),
            );
            let _ = cr.show_text(&label);
        }
    }
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();

    let screen_group = ndl::PreferencesGroup::builder()
        .title(tr("Screens"))
        .build();
    screen_group.set_visible(false);
    let message = ndl::ActionRow::builder()
        .title(tr("Looking for screens…"))
        .build();
    message.add_prefix(&gtk::Image::from_icon_name("video-display-symbolic"));

    let canvas = gtk::DrawingArea::new();
    canvas.set_content_height(200);
    canvas.set_hexpand(true);
    let arrange_group = ndl::PreferencesGroup::builder()
        .title(tr("Arrangement"))
        .description(tr("Drag the screens to where they are on your desk"))
        .build();
    arrange_group.set_visible(false);
    arrange_group.add(&canvas);

    let screen = ndl::ComboRow::builder().title(tr("Screen")).build();
    screen.add_prefix(&gtk::Image::from_icon_name("video-display-symbolic"));
    let resolution = ndl::ComboRow::builder().title(tr("Resolution")).build();
    resolution.add_prefix(&gtk::Image::from_icon_name("view-fullscreen-symbolic"));
    let refresh = ndl::ComboRow::builder().title(tr("Refresh rate")).build();
    refresh.add_prefix(&gtk::Image::from_icon_name("view-refresh-symbolic"));
    let vrr = ndl::SwitchRow::builder()
        .title(tr("Adaptive refresh rate"))
        .subtitle(tr(
            "The screen follows the frame rate of games and videos (FreeSync, G-Sync Compatible)",
        ))
        .build();
    vrr.add_prefix(&gtk::Image::from_icon_name(
        "media-playlist-shuffle-symbolic",
    ));
    let vrr_feature = ndl::SwitchRow::builder()
        .title(tr("Offer adaptive refresh rate"))
        .subtitle(tr(
            "Experimental in the desktop; the screens that support it then show the option above",
        ))
        .build();
    vrr_feature.add_prefix(&gtk::Image::from_icon_name("applications-science-symbolic"));
    vrr_feature.set_visible(false);
    let scale = ndl::ComboRow::builder().title(tr("Scale")).build();
    scale.add_prefix(&gtk::Image::from_icon_name("zoom-in-symbolic"));
    let labels: Vec<&str> = ORIENTATIONS.iter().map(|l| tr(l)).collect();
    let orientation = ndl::ComboRow::builder()
        .title(tr("Orientation"))
        .model(&gtk::StringList::new(&labels))
        .build();
    orientation.add_prefix(&gtk::Image::from_icon_name("object-rotate-right-symbolic"));
    let primary = ndl::SwitchRow::builder()
        .title(tr("Main screen"))
        .subtitle(tr("The top bar and new windows go here"))
        .build();
    primary.add_prefix(&gtk::Image::from_icon_name("starred-symbolic"));
    let off_row = ndl::ActionRow::builder()
        .title(tr("Use this screen"))
        .build();
    off_row.add_prefix(&gtk::Image::from_icon_name("system-shutdown-symbolic"));
    let off = gtk::Button::with_label(tr("Turn off"));
    off.set_valign(gtk::Align::Center);
    off_row.add_suffix(&off);
    screen_group.add(&message);
    screen_group.add(&screen);
    screen_group.add(&resolution);
    screen_group.add(&refresh);
    screen_group.add(&vrr);
    screen_group.add(&scale);
    screen_group.add(&orientation);
    screen_group.add(&primary);
    screen_group.add(&off_row);
    screen_group.add(&vrr_feature);

    let disabled_group = ndl::PreferencesGroup::builder()
        .title(tr("Screens that are off"))
        .build();
    disabled_group.set_visible(false);

    let apply = gtk::Button::with_label(tr("Apply"));
    apply.add_css_class("suggested-action");
    apply.set_sensitive(false);
    let revert = gtk::Button::with_label(tr("Revert"));
    revert.set_sensitive(false);
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    bar.set_halign(gtk::Align::End);
    bar.append(&revert);
    bar.append(&apply);

    let page = Rc::new(Page {
        widget: scroller(&content),
        toasts: toasts.clone(),
        original: RefCell::default(),
        edited: RefCell::default(),
        selected: Cell::new(0),
        loading: Cell::new(true),
        busy: Cell::new(false),
        canvas: canvas.clone(),
        drag: RefCell::default(),
        screen_group: screen_group.clone(),
        screen: screen.clone(),
        resolution: resolution.clone(),
        refresh: refresh.clone(),
        vrr: vrr.clone(),
        vrr_feature: vrr_feature.clone(),
        scale: scale.clone(),
        orientation: orientation.clone(),
        primary: primary.clone(),
        off: off.clone(),
        off_row: off_row.clone(),
        disabled_group: disabled_group.clone(),
        disabled_rows: RefCell::default(),
        arrange_group: arrange_group.clone(),
        apply: apply.clone(),
        revert: revert.clone(),
        message,
    });

    content.append(&arrange_group);
    content.append(&screen_group);
    content.append(&disabled_group);
    content.append(&bar);

    // The drawing and the dragging of the screens.
    {
        let page = page.clone();
        canvas.set_draw_func(move |area, cr, w, h| draw(&page, area, cr, w, h));
    }
    let drag = gtk::GestureDrag::new();
    {
        let page = page.clone();
        drag.connect_drag_begin(move |_, x, y| {
            let Some(config) = page.config() else { return };
            let (k, ox, oy) = view(
                &config,
                f64::from(page.canvas.width()),
                f64::from(page.canvas.height()),
            );
            let hit = (0..config.logicals.len()).rev().find(|&i| {
                let (w, h) = config.size(i);
                let l = &config.logicals[i];
                let (rx, ry) = (ox + f64::from(l.x) * k, oy + f64::from(l.y) * k);
                x >= rx && x <= rx + f64::from(w) * k && y >= ry && y <= ry + f64::from(h) * k
            });
            if let Some(index) = hit {
                let l = &config.logicals[index];
                page.selected.set(index);
                *page.drag.borrow_mut() = Some(Drag {
                    index,
                    start: (l.x, l.y),
                    offset: (0.0, 0.0),
                });
                page.render();
            }
        });
    }
    {
        let page = page.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            if let Some(d) = page.drag.borrow_mut().as_mut() {
                d.offset = (dx, dy);
            }
            page.canvas.queue_draw();
        });
    }
    {
        let page = page.clone();
        drag.connect_drag_end(move |_, dx, dy| {
            let Some(d) = page.drag.borrow_mut().take() else {
                return;
            };
            let Some(config) = page.config() else { return };
            let (k, _, _) = view(
                &config,
                f64::from(page.canvas.width()),
                f64::from(page.canvas.height()),
            );
            let (x, y) = (
                d.start.0 + (dx / k).round() as i32,
                d.start.1 + (dy / k).round() as i32,
            );
            let index = d.index;
            page.selected.set(index);
            page.edit(move |c, _| c.move_to(index, x, y));
        });
    }
    canvas.add_controller(drag);

    // The rows.
    {
        let page = page.clone();
        screen.connect_selected_notify(move |row| {
            if page.loading.get() {
                return;
            }
            page.selected.set(row.selected() as usize);
            page.render();
        });
    }
    {
        let page = page.clone();
        resolution.connect_selected_notify(move |row| {
            let pos = row.selected() as usize;
            page.edit(move |c, i| {
                if let Some(size) = c.resolutions(i).get(pos).copied() {
                    c.set_resolution(i, size);
                }
            });
        });
    }
    {
        let page = page.clone();
        refresh.connect_selected_notify(move |row| {
            let pos = row.selected() as usize;
            page.edit(move |c, i| {
                let Some(size) = c.mode_of(i).map(|m| (m.width, m.height)) else {
                    return;
                };
                if let Some(id) = c.refresh_rates(i, size).get(pos).map(|m| m.id.clone()) {
                    c.set_mode(i, &id);
                }
            });
        });
    }
    {
        let page = page.clone();
        vrr.connect_active_notify(move |row| {
            let on = row.is_active();
            // The page setting the switch to what the configuration says must not edit again.
            let index = page.selected.get();
            if page.config().is_some_and(|c| c.vrr_on(index) == on) {
                return;
            }
            page.edit(move |c, i| {
                c.set_vrr(i, on);
            });
        });
    }
    {
        let page = page.clone();
        vrr_feature.connect_active_notify(move |row| {
            if page.loading.get() {
                return;
            }
            set_experimental_vrr(row.is_active());
            // The desktop reloads its screens a moment after the option changes.
            let page = page.clone();
            glib::timeout_add_seconds_local_once(2, move || page.load());
        });
    }
    {
        let page = page.clone();
        scale.connect_selected_notify(move |row| {
            let pos = row.selected() as usize;
            page.edit(move |c, i| {
                if let Some(s) = c.scales(i).get(pos).copied() {
                    c.set_scale(i, s);
                }
            });
        });
    }
    {
        let page = page.clone();
        orientation.connect_selected_notify(move |row| {
            let t = row.selected();
            page.edit(move |c, i| {
                c.set_transform(i, t);
            });
        });
    }
    {
        let page = page.clone();
        primary.connect_active_notify(move |row| {
            let index = page.selected.get();
            if row.is_active() && page.config().is_some_and(|c| !c.logicals[index].primary) {
                page.edit(|c, i| c.set_primary(i));
            }
        });
    }
    {
        let page = page.clone();
        off.connect_clicked(move |_| {
            page.edit(|c, i| {
                c.disable(i);
            });
        });
    }
    {
        let page = page.clone();
        apply.connect_clicked(move |_| page.apply_edited());
    }
    {
        let page = page.clone();
        revert.connect_clicked(move |_| {
            let original = page.original.borrow().clone();
            *page.edited.borrow_mut() = original;
            page.selected.set(0);
            page.render();
        });
    }

    // Night Light.
    let night = gs::open(COLOR);
    if let Some(group) = night.as_ref().and_then(|g| {
        gs::group(
            tr("Night Light"),
            rows![
                g.switch(
                    "night-light-enabled",
                    tr("Turn on"),
                    tr("Warmer colors in the evening")
                ),
                g.switch(
                    "night-light-schedule-automatic",
                    tr("Sunset to sunrise"),
                    ""
                ),
                g.spin(
                    "night-light-schedule-from",
                    tr("From (hour)"),
                    (0.0, 23.5, 0.5)
                ),
                g.spin("night-light-schedule-to", tr("To (hour)"), (0.0, 23.5, 0.5)),
                g.slider(
                    "night-light-temperature",
                    tr("Color temperature"),
                    tr("Warmer to the right"),
                    (1700.0, 4700.0, 100.0)
                ),
            ],
        )
    }) {
        content.append(&group);
    }

    page.load();
    page.widget.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(
        id: &str,
        w: i32,
        h: i32,
        r: f64,
        current: bool,
    ) -> (
        String,
        i32,
        i32,
        f64,
        f64,
        Vec<f64>,
        HashMap<String, glib::Variant>,
    ) {
        let mut props = HashMap::new();
        if current {
            props.insert("is-current".to_owned(), true.to_variant());
        }
        (id.to_owned(), w, h, r, 1.0, vec![1.0, 2.0], props)
    }

    fn state() -> glib::Variant {
        let ids = |c: &str| {
            (
                c.to_owned(),
                "DEL".to_owned(),
                "U2720Q".to_owned(),
                "1".to_owned(),
            )
        };
        let mut monitor_props = HashMap::new();
        monitor_props.insert("display-name".to_owned(), "Dell U2720Q".to_variant());
        let monitors = vec![(
            ids("DP-1"),
            vec![
                mode("m4k", 3840, 2160, 60.0, false),
                mode("mhd", 1920, 1080, 59.94, true),
            ],
            monitor_props,
        )];
        let logical = vec![(
            0i32,
            0i32,
            1.25f64,
            0u32,
            true,
            vec![ids("DP-1")],
            HashMap::<String, glib::Variant>::new(),
        )];
        (
            7u32,
            monitors,
            logical,
            HashMap::<String, glib::Variant>::new(),
        )
            .to_variant()
    }

    #[test]
    fn the_screen_in_use_and_its_mode_are_read() {
        let config = parse_state(&state()).expect("a configuration");
        assert_eq!(config.serial, 7);
        assert_eq!(config.monitors[0].name, "Dell U2720Q");
        assert_eq!(
            config.logicals[0].monitors,
            vec![("DP-1".to_owned(), "mhd".to_owned())]
        );
        assert_eq!(config.logicals[0].scale, 1.25);
        assert!(config.logicals[0].primary);
    }

    #[test]
    fn a_reply_of_another_shape_gives_nothing() {
        assert!(parse_state(&"nothing".to_variant()).is_none());
    }

    #[test]
    fn what_is_sent_has_the_shape_mutter_expects() {
        let config = parse_state(&state()).expect("a configuration");
        let args = apply_arguments(&config, 9, 1);
        assert_eq!(args.type_().as_str(), "(uua(iiduba(ssa{sv}))a{sv})");
        assert_eq!(args.child_value(0).get::<u32>(), Some(9));
    }

    #[test]
    fn a_rate_shows_decimals_only_when_it_has_them() {
        assert_eq!(rate_text(60.0), "60 Hz");
        assert_eq!(rate_text(59.94), "59.94 Hz");
        assert_eq!(rate_text(119.9999), "120 Hz");
    }
}
