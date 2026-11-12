//! Keyboard: key repeat, input sources (layouts), the compose key and the shortcuts.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ajustes_core::keyboard::{self as kb, Layout};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gdk, gio, glib, prelude::*},
};

use super::{column, gs, scroller};
use crate::i18n::tr;
use crate::rows;

const XKB_LIST: &str = "/usr/share/X11/xkb/rules/evdev.lst";
const SOURCES: &str = "org.gnome.desktop.input-sources";

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let (typing, more) = typing_groups();
    if let Some(g) = typing {
        content.append(&g);
    }
    if let Some(g) = sources_group() {
        content.append(&g);
    }
    if let Some(g) = more {
        content.append(&g);
    }
    for g in shortcut_groups(toasts) {
        content.append(&g);
    }
    scroller(&content)
}

fn typing_groups() -> (Option<ndl::PreferencesGroup>, Option<ndl::PreferencesGroup>) {
    let keyboard = gs::open("org.gnome.desktop.peripherals.keyboard");
    let typing = keyboard.as_ref().and_then(|g| {
        gs::group(
            tr("Typing"),
            rows![
                g.switch(
                    "repeat",
                    tr("Repeat keys"),
                    tr("Hold a key to type it again")
                ),
                g.spin(
                    "delay",
                    tr("Delay before repeating (ms)"),
                    (100.0, 2000.0, 50.0)
                ),
                g.spin(
                    "repeat-interval",
                    tr("Time between repeats (ms)"),
                    (10.0, 300.0, 5.0)
                ),
                g.switch("remember-numlock-state", tr("Remember Num Lock"), ""),
            ],
        )
    });
    let compose = gs::open(SOURCES).and_then(|g| compose_row(&g));
    (typing, gs::group("", vec![compose.map(|r| r.upcast())]))
}

fn compose_row(g: &gs::Gs) -> Option<ndl::ComboRow> {
    if !g.has("xkb-options") {
        return None;
    }
    let settings = g.settings().clone();
    let labels: Vec<String> = kb::COMPOSE_CHOICES
        .iter()
        .map(|(label, _)| tr(label).to_string())
        .collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let row = ndl::ComboRow::builder()
        .title(tr("Compose key"))
        .subtitle(tr("Hold it and type letters to make accents and symbols"))
        .model(&gtk::StringList::new(&labels))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("input-keyboard-symbolic"));
    let selected = |settings: &gio::Settings| {
        let options: Vec<String> = settings
            .strv("xkb-options")
            .iter()
            .map(|s| s.to_string())
            .collect();
        let current = kb::compose_of(&options).unwrap_or("");
        kb::COMPOSE_CHOICES
            .iter()
            .position(|(_, option)| *option == current)
            .unwrap_or(0) as u32
    };
    row.set_selected(selected(&settings));
    row.set_sensitive(settings.is_writable("xkb-options"));
    {
        let (row, settings) = (row.clone(), settings.clone());
        settings
            .clone()
            .connect_changed(Some("xkb-options"), move |s, _| {
                let wanted = selected(s);
                if row.selected() != wanted {
                    row.set_selected(wanted);
                }
            });
    }
    row.connect_selected_notify(move |row| {
        let Some((_, choice)) = kb::COMPOSE_CHOICES.get(row.selected() as usize) else {
            return;
        };
        let options: Vec<String> = settings
            .strv("xkb-options")
            .iter()
            .map(|s| s.to_string())
            .collect();
        if kb::compose_of(&options).unwrap_or("") == *choice {
            return;
        }
        let new = kb::with_compose(&options, choice);
        let new: Vec<&str> = new.iter().map(String::as_str).collect();
        let _ = settings.set_strv("xkb-options", new);
    });
    Some(row)
}

/// The layouts in use, in order, with a way to move, remove and add.
struct Sources {
    settings: gio::Settings,
    layouts: Vec<Layout>,
    group: ndl::PreferencesGroup,
    rows: RefCell<Vec<ndl::ActionRow>>,
}

impl Sources {
    fn read(&self) -> Vec<(String, String)> {
        self.settings.get::<Vec<(String, String)>>("sources")
    }

    fn write(&self, sources: &[(String, String)]) {
        let _ = self.settings.set_value("sources", &sources.to_variant());
    }

    fn render(self: &Rc<Self>) {
        for old in self.rows.borrow_mut().drain(..) {
            self.group.remove(&old);
        }
        let sources = self.read();
        let writable = self.settings.is_writable("sources");
        let mut rows = Vec::new();
        for (i, (kind, id)) in sources.iter().enumerate() {
            let row = ndl::ActionRow::builder()
                .title(if kind == "xkb" {
                    kb::describe(&self.layouts, id)
                } else {
                    id.clone()
                })
                .subtitle(if i == 0 { tr("In use first") } else { "" })
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("input-keyboard-symbolic"));
            let tools = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            tools.set_valign(gtk::Align::Center);
            if i > 0 {
                let up = gtk::Button::from_icon_name("go-up-symbolic");
                up.set_tooltip_text(Some(tr("Move up")));
                up.add_css_class("flat");
                up.set_sensitive(writable);
                let me = self.clone();
                up.connect_clicked(move |_| {
                    let mut now = me.read();
                    kb::move_up(&mut now, i);
                    me.write(&now);
                });
                tools.append(&up);
            }
            if sources.len() > 1 {
                let remove = gtk::Button::from_icon_name("user-trash-symbolic");
                remove.set_tooltip_text(Some(tr("Remove")));
                remove.add_css_class("flat");
                remove.set_sensitive(writable);
                let me = self.clone();
                remove.connect_clicked(move |_| {
                    let mut now = me.read();
                    if i < now.len() && now.len() > 1 {
                        now.remove(i);
                        me.write(&now);
                    }
                });
                tools.append(&remove);
            }
            row.add_suffix(&tools);
            self.group.add(&row);
            rows.push(row);
        }
        *self.rows.borrow_mut() = rows;
    }
}

fn sources_group() -> Option<ndl::PreferencesGroup> {
    let g = gs::open(SOURCES).filter(|g| g.has("sources"))?;
    let layouts = std::fs::read_to_string(XKB_LIST)
        .map(|t| kb::parse_layouts(&t))
        .unwrap_or_default();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Input sources"))
        .description(tr(
            "The first one is used when you sign in; switch with Super+Space",
        ))
        .build();
    let me = Rc::new(Sources {
        settings: g.settings().clone(),
        layouts,
        group: group.clone(),
        rows: RefCell::default(),
    });
    me.render();
    {
        let me = me.clone();
        me.settings
            .clone()
            .connect_changed(Some("sources"), move |_, _| me.render());
    }
    if me.layouts.is_empty() || !me.settings.is_writable("sources") {
        return Some(group);
    }

    let names: Vec<&str> = me.layouts.iter().map(|l| l.name.as_str()).collect();
    let layout = ndl::ComboRow::builder()
        .title(tr("Add a layout"))
        .model(&gtk::StringList::new(&names))
        .build();
    layout.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
    let variant = ndl::ComboRow::builder().title(tr("Variant")).build();
    let fill = {
        let (me, layout, variant) = (me.clone(), layout.clone(), variant.clone());
        move || {
            let mut labels = vec![tr("Standard").to_owned()];
            if let Some(l) = me.layouts.get(layout.selected() as usize) {
                labels.extend(l.variants.iter().map(|v| v.name.clone()));
            }
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            variant.set_model(Some(&gtk::StringList::new(&labels)));
            variant.set_selected(0);
        }
    };
    fill();
    layout.connect_selected_notify(move |_| fill());
    let add = gtk::Button::with_label(tr("Add"));
    add.add_css_class("suggested-action");
    add.set_valign(gtk::Align::Center);
    variant.add_suffix(&add);
    {
        let (me, layout, variant) = (me.clone(), layout.clone(), variant.clone());
        add.connect_clicked(move |_| {
            let Some(l) = me.layouts.get(layout.selected() as usize) else {
                return;
            };
            let chosen = variant.selected() as usize;
            let code = if chosen == 0 {
                String::new()
            } else {
                l.variants
                    .get(chosen - 1)
                    .map(|v| v.code.clone())
                    .unwrap_or_default()
            };
            let id = kb::source_id(&l.code, &code);
            let mut now = me.read();
            if now.iter().any(|(_, existing)| *existing == id) {
                return;
            }
            now.push(("xkb".to_owned(), id));
            me.write(&now);
        });
    }
    group.add(&layout);
    group.add(&variant);
    Some(group)
}

fn is_modifier(key: gdk::Key) -> bool {
    use gdk::Key as K;
    [
        K::Shift_L,
        K::Shift_R,
        K::Control_L,
        K::Control_R,
        K::Alt_L,
        K::Alt_R,
        K::Super_L,
        K::Super_R,
        K::Meta_L,
        K::Meta_R,
        K::Hyper_L,
        K::Hyper_R,
        K::Caps_Lock,
        K::Num_Lock,
        K::ISO_Level3_Shift,
        K::ISO_Level5_Shift,
    ]
    .contains(&key)
}

/// The accelerators stored in a key, whether it holds a list or a single string.
fn accelerators(settings: &gio::Settings, key: &str) -> Vec<String> {
    let value = settings.value(key);
    if value.type_().is_array() {
        settings.strv(key).iter().map(|s| s.to_string()).collect()
    } else {
        let one = settings.string(key).to_string();
        if one.is_empty() || one == "disabled" {
            Vec::new()
        } else {
            vec![one]
        }
    }
}

fn store_accelerator(settings: &gio::Settings, key: &str, accelerator: Option<&str>) {
    if settings.value(key).type_().is_array() {
        let list: Vec<&str> = accelerator.into_iter().collect();
        let _ = settings.set_strv(key, list);
    } else {
        let _ = settings.set_string(key, accelerator.unwrap_or(""));
    }
}

fn labels_of(accelerators: &[String]) -> Vec<String> {
    accelerators
        .iter()
        .filter_map(gtk::accelerator_parse)
        .map(|(key, mods)| gtk::accelerator_get_label(key, mods).to_string())
        .collect()
}

/// The first accelerator, which is the one a person sees; the rest are in the tooltip.
fn label_of(accelerators: &[String]) -> String {
    labels_of(accelerators)
        .into_iter()
        .next()
        .unwrap_or_else(|| tr("Disabled").to_owned())
}

/// A key that stands alone without a modifier: function keys, media keys, Print.
fn stands_alone(key: gdk::Key) -> bool {
    let name = key.name().map(|n| n.to_string()).unwrap_or_default();
    name.starts_with("XF86")
        || name == "Print"
        || (name.starts_with('F') && name[1..].chars().all(|c| c.is_ascii_digit()))
}

struct ShortcutRows {
    entries: Vec<(&'static str, gio::Settings, &'static str)>,
}

impl ShortcutRows {
    fn taken(&self) -> Vec<(&'static str, Vec<String>)> {
        self.entries
            .iter()
            .map(|(title, settings, key)| (*title, accelerators(settings, key)))
            .collect()
    }
}

fn shortcut_groups(toasts: &ndl::ToastOverlay) -> Vec<ndl::PreferencesGroup> {
    let mut opened: HashMap<&str, Option<gs::Gs>> = HashMap::new();
    let mut found: Vec<(&kb::Shortcut, gio::Settings)> = Vec::new();
    let table = kb::shortcuts();
    for s in &table {
        let g = opened.entry(s.schema).or_insert_with(|| gs::open(s.schema));
        if let Some(g) = g.as_ref().filter(|g| g.has(s.key)) {
            found.push((s, g.settings().clone()));
        }
    }
    let all = Rc::new(ShortcutRows {
        entries: found
            .iter()
            .map(|(s, settings)| (s.title, settings.clone(), s.key))
            .collect(),
    });
    let mut groups: Vec<ndl::PreferencesGroup> = Vec::new();
    let mut current = "";
    for (index, (s, settings)) in found.iter().enumerate() {
        if s.group != current {
            current = s.group;
            groups.push(
                ndl::PreferencesGroup::builder()
                    .title(format!("{} · {}", tr("Shortcuts"), tr(s.group)))
                    .build(),
            );
        }
        let row = ndl::ActionRow::builder().title(tr(s.title)).build();
        let button = gtk::Button::new();
        button.set_valign(gtk::Align::Center);
        button.add_css_class("flat");
        button.set_sensitive(settings.is_writable(s.key));
        let show = {
            let (button, settings, key) = (button.clone(), settings.clone(), s.key);
            move || {
                let all = accelerators(&settings, key);
                button.set_label(&label_of(&all));
                let labels = labels_of(&all);
                button.set_tooltip_text((labels.len() > 1).then(|| labels.join(", ")).as_deref());
            }
        };
        show();
        {
            let show = show.clone();
            settings.connect_changed(Some(s.key), move |_, _| show());
        }
        let capturing = Rc::new(Cell::new(false));
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let (capturing, show, settings, all, toasts) = (
                capturing.clone(),
                show.clone(),
                settings.clone(),
                all.clone(),
                toasts.clone(),
            );
            let key_name = s.key;
            keys.connect_key_pressed(move |_, key, _, state| {
                if !capturing.get() {
                    return glib::Propagation::Proceed;
                }
                let mods = state & gtk::accelerator_get_default_mod_mask();
                if is_modifier(key) {
                    return glib::Propagation::Stop;
                }
                if mods.is_empty() && key == gdk::Key::Escape {
                    capturing.set(false);
                    show();
                    return glib::Propagation::Stop;
                }
                if mods.is_empty() && key == gdk::Key::BackSpace {
                    capturing.set(false);
                    store_accelerator(&settings, key_name, None);
                    show();
                    return glib::Propagation::Stop;
                }
                if mods.is_empty() && !stands_alone(key) {
                    toasts.add_toast(ndl::Toast::new(tr(
                        "Use Ctrl, Alt, Shift or Super with another key",
                    )));
                    return glib::Propagation::Stop;
                }
                let accelerator = gtk::accelerator_name(key.to_lower(), mods).to_string();
                if let Some(other) = kb::conflict(&all.taken(), index, &accelerator) {
                    let message = format!("{} {}", tr("Already used by"), tr(other));
                    toasts.add_toast(ndl::Toast::new(&message));
                    return glib::Propagation::Stop;
                }
                capturing.set(false);
                store_accelerator(&settings, key_name, Some(&accelerator));
                show();
                glib::Propagation::Stop
            });
        }
        button.add_controller(keys);
        let leave = gtk::EventControllerFocus::new();
        {
            let (capturing, show) = (capturing.clone(), show.clone());
            leave.connect_leave(move |_| {
                if capturing.replace(false) {
                    show();
                }
            });
        }
        button.add_controller(leave);
        button.connect_clicked(move |b| {
            capturing.set(true);
            b.set_label(tr("Press a shortcut…"));
            b.grab_focus();
        });
        row.add_suffix(&button);
        if let Some(group) = groups.last() {
            group.add(&row);
        }
    }
    groups
}
