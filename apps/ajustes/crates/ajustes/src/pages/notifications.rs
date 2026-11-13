//! Notifications: banners, the lock screen and, for each app that has sent one, whether it may.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, prelude::*},
};

use super::gs;
use crate::i18n::tr;

const SCHEMA: &str = "org.gnome.desktop.notifications";
const APP_SCHEMA: &str = "org.gnome.desktop.notifications.application";

/// A switch in a popover, bound to a boolean key.
fn popover_switch(app: &gs::Gs, key: &str, label: &str) -> Option<gtk::Box> {
    if !app.has(key) {
        return None;
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let text = gtk::Label::new(Some(label));
    text.set_hexpand(true);
    text.set_xalign(0.0);
    let switch = gtk::Switch::new();
    switch.set_valign(gtk::Align::Center);
    app.settings().bind(key, &switch, "active").build();
    switch.set_sensitive(app.settings().is_writable(key));
    row.append(&text);
    row.append(&switch);
    Some(row)
}

/// The apps that have sent a notification, one row each: a switch for the app and, beside it,
/// what it may do. An app that is no longer installed is left out: its entry is stale.
fn app_rows(main: &gs::Gs) -> Vec<(String, ndl::ActionRow)> {
    let installed: std::collections::HashMap<String, gio::AppInfo> = gio::AppInfo::all()
        .into_iter()
        .filter_map(|app| Some((app.id()?.to_string(), app)))
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
        let Some(info) = installed.get(id.as_str()) else {
            continue;
        };
        let name = info.name().to_string();
        let row = ndl::ActionRow::builder().title(&name).build();
        let icon = match info.icon() {
            Some(icon) => gtk::Image::from_gicon(&icon),
            None => gtk::Image::from_icon_name("application-x-executable-symbolic"),
        };
        icon.set_pixel_size(24);
        row.add_prefix(&icon);

        let options = gtk::Box::new(gtk::Orientation::Vertical, 10);
        options.set_margin_top(12);
        options.set_margin_bottom(12);
        options.set_margin_start(12);
        options.set_margin_end(12);
        options.set_size_request(280, -1);
        for (key, label) in [
            ("show-banners", tr("Show banners")),
            ("enable-sound-alerts", tr("Play sounds")),
            ("show-in-lock-screen", tr("Show on the lock screen")),
            (
                "details-in-lock-screen",
                tr("Show the details on the lock screen"),
            ),
            ("force-expanded", tr("Always show the whole message")),
        ] {
            if let Some(line) = popover_switch(&app, key, label) {
                options.append(&line);
            }
        }
        let more = gtk::MenuButton::new();
        more.set_icon_name("emblem-system-symbolic");
        more.set_tooltip_text(Some(tr("What this app may do")));
        more.set_valign(gtk::Align::Center);
        more.add_css_class("flat");
        let popover = gtk::Popover::new();
        popover.set_child(Some(&options));
        more.set_popover(Some(&popover));
        row.add_suffix(&more);

        let switch = gtk::Switch::new();
        switch.set_valign(gtk::Align::Center);
        app.settings().bind("enable", &switch, "active").build();
        switch.set_sensitive(app.settings().is_writable("enable"));
        // The options only matter while the app may notify at all.
        switch
            .bind_property("active", &more, "sensitive")
            .sync_create()
            .build();
        row.add_suffix(&switch);
        row.set_activatable_widget(Some(&switch));
        apps.push((name, row));
    }
    apps.sort_by_key(|(name, _)| name.to_lowercase());
    apps
}

pub fn build() -> gtk::ScrolledWindow {
    let main = gs::open(SCHEMA);
    let general = main.as_ref().and_then(|m| {
        if !m.has("show-banners") {
            return None;
        }
        let group = ndl::PreferencesGroup::new();
        // Do Not Disturb is the banners switched off: the desktop has no separate setting.
        let quiet = ndl::SwitchRow::builder()
            .title(tr("Do not disturb"))
            .subtitle(tr("Hide the banners; notifications still wait in the list"))
            .build();
        quiet.add_prefix(&gtk::Image::from_icon_name(
            "notifications-disabled-symbolic",
        ));
        m.settings()
            .bind("show-banners", &quiet, "active")
            .flags(gio::SettingsBindFlags::INVERT_BOOLEAN)
            .build();
        quiet.set_sensitive(m.settings().is_writable("show-banners"));
        group.add(&quiet);
        if let Some(lock) = m.switch(
            "show-in-lock-screen",
            tr("Notifications on the lock screen"),
            "",
        ) {
            group.add(&lock);
        }
        Some(group)
    });
    let apps = main.as_ref().map(|m| {
        let apps = app_rows(m);
        let group = ndl::PreferencesGroup::builder()
            .title(tr("Apps"))
            .description(tr("Choose which apps may notify you."))
            .build();
        if apps.is_empty() {
            group.add(
                &ndl::ActionRow::builder()
                    .title(tr("No app has sent a notification yet"))
                    .subtitle(tr("Apps appear here the first time they notify you"))
                    .build(),
            );
        }
        for (_, row) in &apps {
            group.add(row);
        }
        group
    });
    gs::page(vec![general, apps])
}
