//! Date and time: automatic time, time zone and how the clock is shown.

use std::cell::Cell;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::gs::{self, s};
use super::{bus, column, scroller};
use crate::i18n::tr;
use crate::rows;

const TIMEDATE: &str = "org.freedesktop.timedate1";
const TIMEDATE_PATH: &str = "/org/freedesktop/timedate1";
const ZONES: &str = "/usr/share/zoneinfo/zone1970.tab";

const FORMAT: [(&str, gs::Val); 2] = [s("24 hours", "24h"), s("AM/PM", "12h")];

/// The time zone names in a `zone1970.tab`: the third column of every line that is not a
/// comment, sorted.
pub fn parse_zones(table: &str) -> Vec<String> {
    let mut zones: Vec<String> = table
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split('\t').nth(2))
        .map(str::to_owned)
        .collect();
    zones.sort();
    zones.dedup();
    zones
}

/// A zone name is `Region/City` with letters, digits and `_+-/` only; anything else is never
/// sent to the system.
pub fn plausible_zone(zone: &str) -> bool {
    !zone.is_empty()
        && zone.len() <= 64
        && !zone.starts_with('/')
        && !zone.contains("..")
        && zone
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_+-/".contains(c))
}

struct System {
    automatic: ndl::SwitchRow,
    zone: ndl::EntryRow,
    apply: gtk::Button,
    saved_zone: std::cell::RefCell<String>,
    loading: Cell<bool>,
    zones: Vec<String>,
}

fn system_group(toasts: &ndl::ToastOverlay) -> ndl::PreferencesGroup {
    let group = ndl::PreferencesGroup::new();
    let ui = Rc::new(System {
        automatic: ndl::SwitchRow::builder()
            .title(tr("Automatic date and time"))
            .subtitle(tr("Set the clock from the internet"))
            .build(),
        zone: {
            let row = ndl::EntryRow::builder().title(tr("Time zone")).build();
            row.add_prefix(&gtk::Image::from_icon_name(
                "preferences-system-time-symbolic",
            ));
            row
        },
        apply: gtk::Button::with_label(tr("Apply")),
        saved_zone: Default::default(),
        loading: Cell::new(true),
        zones: std::fs::read_to_string(ZONES)
            .map(|t| parse_zones(&t))
            .unwrap_or_default(),
    });
    ui.automatic
        .add_prefix(&gtk::Image::from_icon_name("network-wireless-symbolic"));
    ui.automatic.set_sensitive(false);
    ui.zone.set_sensitive(false);
    ui.apply.set_valign(gtk::Align::Center);
    ui.apply.set_sensitive(false);
    ui.zone.add_suffix(&ui.apply);
    group.add(&ui.automatic);
    group.add(&ui.zone);

    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let Some(proxy) = bus::proxy(gio::BusType::System, TIMEDATE, TIMEDATE_PATH, TIMEDATE).await
        else {
            ui.automatic
                .set_subtitle(tr("The time service is not available."));
            return;
        };
        let sync = {
            let (ui, proxy) = (ui.clone(), proxy.clone());
            move || {
                ui.loading.set(true);
                ui.automatic
                    .set_active(bus::bool_property(&proxy, "NTP").unwrap_or(false));
                ui.automatic
                    .set_sensitive(bus::bool_property(&proxy, "CanNTP").unwrap_or(false));
                let zone = bus::string_property(&proxy, "Timezone").unwrap_or_default();
                if *ui.saved_zone.borrow() != zone {
                    ui.zone.set_text(&zone);
                    *ui.saved_zone.borrow_mut() = zone;
                }
                ui.zone.set_sensitive(true);
                ui.apply.set_sensitive(false);
                ui.loading.set(false);
            }
        };
        sync();
        bus::on_change(&proxy, {
            let sync = sync.clone();
            move || sync()
        });
        let report = {
            let toasts = toasts.clone();
            move |error: glib::Error| toasts.add_toast(ndl::Toast::new(error.message()))
        };
        {
            let (ui, proxy, sync, report) =
                (ui.clone(), proxy.clone(), sync.clone(), report.clone());
            ui.clone().automatic.connect_active_notify(move |row| {
                if ui.loading.get() {
                    return;
                }
                let (proxy, sync, report, active) =
                    (proxy.clone(), sync.clone(), report.clone(), row.is_active());
                glib::spawn_future_local(async move {
                    let args = (active, true).to_variant();
                    if let Err(error) = proxy
                        .call_future("SetNTP", Some(&args), gio::DBusCallFlags::NONE, -1)
                        .await
                    {
                        report(error);
                        sync();
                    }
                });
            });
        }
        {
            let ui2 = ui.clone();
            ui.zone.connect_changed(move |row| {
                if ui2.loading.get() {
                    return;
                }
                let text = row.text().to_string();
                ui2.apply
                    .set_sensitive(text != *ui2.saved_zone.borrow() && ui2.zones.contains(&text));
            });
        }
        let ui2 = ui.clone();
        ui.apply.connect_clicked(move |_| {
            let zone = ui2.zone.text().to_string();
            if !plausible_zone(&zone) || !ui2.zones.contains(&zone) {
                return;
            }
            let (proxy, sync, report) = (proxy.clone(), sync.clone(), report.clone());
            glib::spawn_future_local(async move {
                let args = (zone, true).to_variant();
                if let Err(error) = proxy
                    .call_future("SetTimezone", Some(&args), gio::DBusCallFlags::NONE, -1)
                    .await
                {
                    report(error);
                    sync();
                }
            });
        });
    });
    group
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    content.append(&system_group(toasts));
    let interface = gs::open("org.gnome.desktop.interface");
    let datetime = gs::open("org.gnome.desktop.datetime");
    if let Some(g) = datetime.as_ref().and_then(|g| {
        gs::group(
            "",
            rows![g.switch(
                "automatic-timezone",
                tr("Automatic time zone"),
                tr("Use your location to set the time zone"),
            )],
        )
    }) {
        content.append(&g);
    }
    if let Some(g) = interface.as_ref().and_then(|g| {
        gs::group(
            tr("Clock"),
            rows![
                g.choice("clock-format", tr("Clock format"), &FORMAT),
                g.switch("clock-show-seconds", tr("Show seconds"), ""),
                g.switch("clock-show-date", tr("Show the date"), ""),
                g.switch("clock-show-weekday", tr("Show the weekday"), ""),
            ],
        )
    }) {
        content.append(&g);
    }
    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_zone_names_of_a_zone_table() {
        let table = "# comment\nES\t+4024+00341\tEurope/Madrid\tMainland\nES\t+2806-01524\tAtlantic/Canary\tCanary\nFR\t+4852+00220\tEurope/Paris\n";
        assert_eq!(
            parse_zones(table),
            ["Atlantic/Canary", "Europe/Madrid", "Europe/Paris"]
        );
    }

    #[test]
    fn only_plausible_zone_names_pass() {
        assert!(plausible_zone("Europe/Madrid"));
        assert!(plausible_zone("America/Argentina/Buenos_Aires"));
        assert!(plausible_zone("Etc/GMT+5"));
        for bad in ["", "/etc/passwd", "../x", "a b", "a;b", "Europe/Madrid\n"] {
            assert!(!plausible_zone(bad), "{bad:?}");
        }
    }

    #[test]
    fn every_label_is_translated() {
        for (label, _) in &FORMAT {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
