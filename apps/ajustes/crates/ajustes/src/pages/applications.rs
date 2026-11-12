//! Apps: which application opens each kind of file or link, and the apps that are installed.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, prelude::*},
};

use ajustes_core::apps::{Default, DEFAULTS};

use super::gs;
use crate::i18n::tr;

/// The apps that can open `content_type`, one per id, in the order GIO lists them.
fn candidates(content_type: &str) -> Vec<gio::AppInfo> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for app in gio::AppInfo::all_for_type(content_type) {
        let Some(id) = app.id() else { continue };
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        out.push(app);
    }
    out
}

fn default_row(kind: &'static Default) -> Option<ndl::ComboRow> {
    let first = kind.types[0];
    let apps = candidates(first);
    if apps.is_empty() {
        // Nothing installed can open this: a row with no choice would only confuse.
        return None;
    }
    let names: Vec<String> = apps.iter().map(|a| a.name().to_string()).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let row = ndl::ComboRow::builder()
        .title(tr(kind.title))
        .model(&gtk::StringList::new(&names))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(kind.icon));
    let current = gio::AppInfo::default_for_type(first, false).and_then(|d| d.id());
    if let Some(pos) = apps.iter().position(|a| a.id() == current) {
        row.set_selected(pos as u32);
    }
    row.connect_selected_notify(move |row| {
        let Some(app) = apps.get(row.selected() as usize) else {
            return;
        };
        let all_set = kind
            .types
            .iter()
            .all(|t| app.set_as_default_for_type(t).is_ok());
        if !all_set {
            // Some type refused (for instance a read-only mimeapps.list): show what is stored.
            let now = gio::AppInfo::default_for_type(first, false).and_then(|d| d.id());
            if let Some(pos) = apps.iter().position(|a| a.id() == now) {
                row.set_selected(pos as u32);
            }
        }
    });
    Some(row)
}

fn installed() -> Option<ndl::PreferencesGroup> {
    let mut apps: Vec<gio::AppInfo> = gio::AppInfo::all()
        .into_iter()
        .filter(|a| a.should_show())
        .collect();
    apps.sort_by_key(|a| a.name().to_lowercase());
    let rows: Vec<Option<gtk::Widget>> = apps
        .iter()
        .map(|app| {
            let row = ndl::ActionRow::builder().title(app.name().as_str()).build();
            if let Some(desc) = app.description().filter(|d| !d.is_empty()) {
                row.set_subtitle(desc.as_str());
            }
            let icon = match app.icon() {
                Some(icon) => gtk::Image::from_gicon(&icon),
                None => gtk::Image::from_icon_name("application-x-executable"),
            };
            icon.set_pixel_size(32);
            row.add_prefix(&icon);
            row.set_activatable(true);
            let app = app.clone();
            row.connect_activated(move |_| {
                // A launch that fails (a broken Exec=) has no one to tell here; the Store and
                // the shell show their own errors for the same file.
                let _ = app.launch(&[], None::<&gio::AppLaunchContext>);
            });
            Some(row.upcast())
        })
        .collect();
    gs::group(tr("Installed apps"), rows)
}

pub fn build() -> gtk::ScrolledWindow {
    let defaults: Vec<Option<gtk::Widget>> = DEFAULTS
        .iter()
        .map(|kind| default_row(kind).map(|r| r.upcast()))
        .collect();
    gs::page(vec![gs::group(tr("Default apps"), defaults), installed()])
}
