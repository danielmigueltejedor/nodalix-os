//! Power: power mode, battery, suspend and the power button.

use std::cell::Cell;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::gs::{self, i, s};
use super::{bus, column, scroller};
use crate::i18n::tr;
use crate::rows;

const POWER: &str = "org.gnome.settings-daemon.plugins.power";
const BUTTON: [(&str, gs::Val); 4] = [
    s("Suspend", "suspend"),
    s("Hibernate", "hibernate"),
    s("Ask what to do", "interactive"),
    s("Do nothing", "nothing"),
];
const SUSPEND: [(&str, gs::Val); 2] = [s("Never", "nothing"), s("Suspend", "suspend")];
const TIMEOUT: [(&str, gs::Val); 7] = [
    i("5 minutes", 300),
    i("15 minutes", 900),
    i("20 minutes", 1200),
    i("30 minutes", 1800),
    i("45 minutes", 2700),
    i("1 hour", 3600),
    i("2 hours", 7200),
];
/// `(label, power-profiles-daemon name)`.
const PROFILES: [(&str, &str); 3] = [
    ("Power saver", "power-saver"),
    ("Balanced", "balanced"),
    ("Performance", "performance"),
];

const PROFILES_BUS: &str = "net.hadess.PowerProfiles";
const PROFILES_PATH: &str = "/net/hadess/PowerProfiles";
const UPOWER: &str = "org.freedesktop.UPower";
const DISPLAY_DEVICE: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const DEVICE_IFACE: &str = "org.freedesktop.UPower.Device";

/// The power mode, from power-profiles-daemon. Hidden when the daemon is not running.
fn profile_row() -> ndl::ComboRow {
    let labels: Vec<&str> = PROFILES.iter().map(|(l, _)| tr(l)).collect();
    let row = ndl::ComboRow::builder()
        .title(tr("Power mode"))
        .model(&gtk::StringList::new(&labels))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(
        "power-profile-balanced-symbolic",
    ));
    row.set_visible(false);
    let loading = Rc::new(Cell::new(true));
    {
        let (row, loading) = (row.clone(), loading.clone());
        glib::spawn_future_local(async move {
            let Some(proxy) = bus::proxy(
                gio::BusType::System,
                PROFILES_BUS,
                PROFILES_PATH,
                PROFILES_BUS,
            )
            .await
            else {
                return;
            };
            let sync = {
                let (row, loading, proxy) = (row.clone(), loading.clone(), proxy.clone());
                move || {
                    let active = bus::string_property(&proxy, "ActiveProfile").unwrap_or_default();
                    let index = PROFILES.iter().position(|(_, p)| *p == active);
                    loading.set(true);
                    if let Some(index) = index {
                        row.set_selected(index as u32);
                    }
                    row.set_visible(index.is_some());
                    loading.set(false);
                }
            };
            sync();
            bus::on_change(&proxy, {
                let sync = sync.clone();
                move || sync()
            });
            row.connect_selected_notify(move |row| {
                if loading.get() {
                    return;
                }
                let Some((_, name)) = PROFILES.get(row.selected() as usize) else {
                    return;
                };
                let (proxy, sync) = (proxy.clone(), sync.clone());
                glib::spawn_future_local(async move {
                    let result = bus::set_property(
                        &proxy,
                        PROFILES_BUS,
                        "ActiveProfile",
                        &name.to_variant(),
                    )
                    .await;
                    if result.is_err() {
                        // Show the mode that is really active.
                        sync();
                    }
                });
            });
        });
    }
    row
}

/// The battery charge, from UPower. Hidden on a machine without a battery.
fn battery_row() -> ndl::ActionRow {
    let row = ndl::ActionRow::builder().title(tr("Battery")).build();
    row.add_prefix(&gtk::Image::from_icon_name("battery-symbolic"));
    row.set_visible(false);
    let label = gtk::Label::new(None);
    row.add_suffix(&label);
    {
        let (row, label) = (row.clone(), label.clone());
        glib::spawn_future_local(async move {
            let Some(proxy) =
                bus::proxy(gio::BusType::System, UPOWER, DISPLAY_DEVICE, DEVICE_IFACE).await
            else {
                return;
            };
            let sync = move |proxy: &gio::DBusProxy| {
                let present = bus::bool_property(proxy, "IsPresent").unwrap_or(false);
                let percent = proxy
                    .cached_property("Percentage")
                    .and_then(|v| v.get::<f64>());
                match percent {
                    Some(p) if present => {
                        label.set_text(&format!("{}%", p.round() as i64));
                        row.set_visible(true);
                    }
                    _ => row.set_visible(false),
                }
            };
            sync(&proxy);
            let watched = proxy.clone();
            bus::on_change(&proxy, move || sync(&watched));
        });
    }
    row
}

pub fn build() -> gtk::ScrolledWindow {
    let power = gs::open(POWER);
    let interface = gs::open("org.gnome.desktop.interface");
    let session = gs::open("org.gnome.desktop.session");

    let content = column();
    let mode = ndl::PreferencesGroup::new();
    mode.add(&profile_row());
    mode.add(&battery_row());
    if let Some(shown) = interface.as_ref().and_then(|g| {
        g.switch(
            "show-battery-percentage",
            tr("Show the battery percentage"),
            "",
        )
    }) {
        mode.add(&shown);
    }
    content.append(&mode);

    let groups = [
        power.as_ref().and_then(|g| {
            gs::group(
                tr("Saving power"),
                rows![
                    g.switch("idle-dim", tr("Dim the screen when idle"), ""),
                    g.switch(
                        "power-saver-profile-on-low-battery",
                        tr("Power saver on low battery"),
                        tr("Switch to the power saver mode when the battery is low"),
                    ),
                    session.as_ref().and_then(|s| s.choice(
                        "idle-delay",
                        tr("Turn the screen off after"),
                        &super::privacy::BLANK,
                    )),
                ],
            )
        }),
        power.as_ref().and_then(|g| {
            gs::group(
                tr("Automatic suspend"),
                rows![
                    g.choice(
                        "sleep-inactive-battery-type",
                        tr("On battery power"),
                        &SUSPEND
                    ),
                    g.choice(
                        "sleep-inactive-battery-timeout",
                        tr("Suspend after (battery)"),
                        &TIMEOUT
                    ),
                    g.choice("sleep-inactive-ac-type", tr("Plugged in"), &SUSPEND),
                    g.choice(
                        "sleep-inactive-ac-timeout",
                        tr("Suspend after (plugged in)"),
                        &TIMEOUT
                    ),
                ],
            )
        }),
        power.as_ref().and_then(|g| {
            gs::group(
                tr("Power button"),
                rows![g.choice(
                    "power-button-action",
                    tr("When the power button is pressed"),
                    &BUTTON
                )],
            )
        }),
    ];
    for group in groups.into_iter().flatten() {
        content.append(&group);
    }
    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_is_translated() {
        for (label, _) in BUTTON.iter().chain(&SUSPEND).chain(&TIMEOUT) {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
        for (label, _) in &PROFILES {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
