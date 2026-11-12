//! Notifications: banners, the lock screen and, for each app that has sent one, whether it may.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, prelude::*},
};

use super::gs;
use crate::i18n::tr;
use crate::rows;

const SCHEMA: &str = "org.gnome.desktop.notifications";
const APP_SCHEMA: &str = "org.gnome.desktop.notifications.application";

/// The apps that have sent a notification, by name. An app that is no longer installed is left
/// out: its entry is stale.
fn app_rows(main: &gs::Gs) -> Vec<(String, ndl::SwitchRow)> {
    let installed: std::collections::HashMap<String, String> = gio::AppInfo::all()
        .iter()
        .filter_map(|app| Some((app.id()?.to_string(), app.name().to_string())))
        .collect();
    let mut apps = Vec::new();
    for child in main.settings().strv("application-children") {
        let path = format!("/org/gnome/desktop/notifications/application/{child}/");
        let Some(app) = gs::open_path(APP_SCHEMA, &path) else {
            continue;
        };
        if !app.has("application-id") || !app.has("enable") {
            continue;
        }
        let id = app.settings().string("application-id");
        let Some(name) = installed.get(id.as_str()).cloned() else {
            continue;
        };
        if let Some(row) = app.switch("enable", &name, "") {
            apps.push((name, row));
        }
    }
    apps.sort_by_key(|(name, _)| name.to_lowercase());
    apps
}

pub fn build() -> gtk::ScrolledWindow {
    let main = gs::open(SCHEMA);
    let general = main.as_ref().and_then(|m| {
        gs::group(
            "",
            rows![
                m.switch(
                    "show-banners",
                    tr("Notification banners"),
                    tr("Show a banner at the top of the screen"),
                ),
                m.switch(
                    "show-in-lock-screen",
                    tr("Notifications on the lock screen"),
                    "",
                ),
            ],
        )
    });
    let apps = main.as_ref().and_then(|m| {
        let apps = app_rows(m);
        if apps.is_empty() {
            return None;
        }
        let group = ndl::PreferencesGroup::builder()
            .title(tr("Apps"))
            .description(tr("Choose which apps may notify you."))
            .build();
        for (_, row) in &apps {
            group.add(row);
        }
        Some(group)
    });
    gs::page(vec![general, apps])
}
