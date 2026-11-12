//! Sound: volume, mute and the default device for output and input, through `pactl`, which
//! PipeWire and PulseAudio both answer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use ajustes_core::audio::{self, Device};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::gs;
use super::{column, scroller};
use crate::i18n::tr;
use crate::rows;

/// Volume changes while a slider moves are sent once it rests.
const DEBOUNCE: Duration = Duration::from_millis(150);
const REFRESH_SECONDS: u32 = 4;

#[derive(Clone, Copy)]
enum Kind {
    Output,
    Input,
}

impl Kind {
    fn list(self) -> &'static str {
        match self {
            Kind::Output => "sinks",
            Kind::Input => "sources",
        }
    }
    fn default_name(self) -> &'static str {
        match self {
            Kind::Output => "get-default-sink",
            Kind::Input => "get-default-source",
        }
    }
    fn set_default(self) -> &'static str {
        match self {
            Kind::Output => "set-default-sink",
            Kind::Input => "set-default-source",
        }
    }
    fn set_volume(self) -> &'static str {
        match self {
            Kind::Output => "set-sink-volume",
            Kind::Input => "set-source-volume",
        }
    }
    fn set_mute(self) -> &'static str {
        match self {
            Kind::Output => "set-sink-mute",
            Kind::Input => "set-source-mute",
        }
    }
    fn default_alias(self) -> &'static str {
        match self {
            Kind::Output => "@DEFAULT_SINK@",
            Kind::Input => "@DEFAULT_SOURCE@",
        }
    }
}

/// Runs `pactl` with these arguments, never through a shell, and returns what it printed.
async fn pactl(args: &[&str]) -> Option<String> {
    let mut argv = vec!["pactl"];
    argv.extend_from_slice(args);
    let process = gio::Subprocess::newv(
        &argv.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
    )
    .ok()?;
    let (out, _) = process.communicate_utf8_future(None).await.ok()?;
    process
        .is_successful()
        .then(|| out.map(|s| s.to_string()))?
}

struct Side {
    kind: Kind,
    device: ndl::ComboRow,
    volume: gtk::Scale,
    mute: ndl::SwitchRow,
    names: RefCell<Vec<String>>,
    /// Writes waiting for the slider to rest.
    pending: Cell<u32>,
    /// The widgets are being set from a reading; their handlers must not write back.
    loading: Cell<bool>,
}

impl Side {
    fn new(kind: Kind) -> Rc<Self> {
        let (device, mute) = match kind {
            Kind::Output => (tr("Output device"), tr("Mute")),
            Kind::Input => (tr("Input device"), tr("Mute the microphone")),
        };
        let side = Rc::new(Side {
            kind,
            device: ndl::ComboRow::builder().title(device).build(),
            volume: gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0),
            mute: ndl::SwitchRow::builder().title(mute).build(),
            names: RefCell::default(),
            pending: Cell::new(0),
            loading: Cell::new(true),
        });
        side.volume.set_width_request(220);
        side.volume.set_valign(gtk::Align::Center);
        side.volume.set_draw_value(true);
        side.volume.set_value_pos(gtk::PositionType::Right);
        for w in [
            side.device.upcast_ref::<gtk::Widget>(),
            side.mute.upcast_ref(),
        ] {
            w.set_sensitive(false);
        }
        side.volume.set_sensitive(false);

        {
            let side2 = side.clone();
            side.device.connect_selected_notify(move |row| {
                if side2.loading.get() {
                    return;
                }
                let Some(name) = side2.names.borrow().get(row.selected() as usize).cloned() else {
                    return;
                };
                let kind = side2.kind;
                glib::spawn_future_local(async move {
                    pactl(&[kind.set_default(), &name]).await;
                });
            });
        }
        {
            let side2 = side.clone();
            side.mute.connect_active_notify(move |row| {
                if side2.loading.get() {
                    return;
                }
                let (kind, on) = (side2.kind, row.is_active());
                glib::spawn_future_local(async move {
                    pactl(&[
                        kind.set_mute(),
                        kind.default_alias(),
                        if on { "1" } else { "0" },
                    ])
                    .await;
                });
            });
        }
        {
            let side2 = side.clone();
            side.volume.connect_value_changed(move |scale| {
                if side2.loading.get() {
                    return;
                }
                side2.pending.set(side2.pending.get() + 1);
                let (side3, percent) = (side2.clone(), scale.value().round() as u32);
                glib::timeout_add_local_once(DEBOUNCE, move || {
                    let left = side3.pending.get().saturating_sub(1);
                    side3.pending.set(left);
                    if left > 0 {
                        return;
                    }
                    let kind = side3.kind;
                    glib::spawn_future_local(async move {
                        pactl(&[
                            kind.set_volume(),
                            kind.default_alias(),
                            &format!("{percent}%"),
                        ])
                        .await;
                    });
                });
            });
        }
        side
    }

    fn group(&self, title: &str) -> ndl::PreferencesGroup {
        let group = ndl::PreferencesGroup::builder().title(title).build();
        group.add(&self.device);
        let volume = ndl::ActionRow::builder().title(tr("Volume")).build();
        volume.add_suffix(&self.volume);
        group.add(&volume);
        group.add(&self.mute);
        group
    }

    fn show(&self, devices: &[Device], default: &str, max: f64) {
        self.loading.set(true);
        let labels: Vec<&str> = devices.iter().map(|d| d.description.as_str()).collect();
        self.device.set_model(Some(&gtk::StringList::new(&labels)));
        *self.names.borrow_mut() = devices.iter().map(|d| d.name.clone()).collect();
        let current = devices.iter().position(|d| d.name == default);
        if let Some(index) = current {
            self.device.set_selected(index as u32);
            self.mute.set_active(devices[index].muted);
            self.volume.set_range(0.0, max);
            if self.pending.get() == 0 {
                self.volume.set_value(f64::from(devices[index].volume));
            }
        }
        for w in [
            self.device.upcast_ref::<gtk::Widget>(),
            self.mute.upcast_ref(),
            self.volume.upcast_ref(),
        ] {
            w.set_sensitive(current.is_some());
        }
        self.loading.set(false);
    }

    async fn refresh(&self, max: f64) {
        let devices = pactl(&["-f", "json", "list", self.kind.list()])
            .await
            .and_then(|json| audio::parse_devices(&json).ok())
            .unwrap_or_default();
        let default = pactl(&[self.kind.default_name()])
            .await
            .map(|s| s.trim().to_owned())
            .unwrap_or_default();
        self.show(&devices, &default, max);
    }
}

pub fn build() -> gtk::ScrolledWindow {
    let content = column();
    let output = Side::new(Kind::Output);
    let input = Side::new(Kind::Input);
    content.append(&output.group(tr("Output")));
    content.append(&input.group(tr("Input")));
    let sound = gs::open("org.gnome.desktop.sound");
    if let Some(g) = sound.as_ref().and_then(|g| {
        gs::group(
            tr("Alerts"),
            rows![
                g.switch("event-sounds", tr("Alert sounds"), ""),
                g.switch(
                    "allow-volume-above-100-percent",
                    tr("Over-amplification"),
                    tr("Allow a volume above 100%")
                ),
            ],
        )
    }) {
        content.append(&g);
    }

    let max = {
        let sound = sound.clone();
        move || match &sound {
            Some(g) if g.boolean("allow-volume-above-100-percent") == Some(true) => 150.0,
            _ => 100.0,
        }
    };
    let refresh = {
        let (output, input) = (output.clone(), input.clone());
        move || {
            let (output, input, max) = (output.clone(), input.clone(), max());
            glib::spawn_future_local(async move {
                output.refresh(max).await;
                input.refresh(max).await;
            });
        }
    };
    refresh();
    // Something else may change the volume (the keys, the quick settings), so look again now and
    // then, but only while this page is on screen.
    let page = scroller(&content);
    {
        let page = page.downgrade();
        glib::timeout_add_seconds_local(REFRESH_SECONDS, move || {
            let Some(page) = page.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if page.is_mapped() {
                refresh();
            }
            glib::ControlFlow::Continue
        });
    }
    page
}
