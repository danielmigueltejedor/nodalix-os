//! Archivador, the Nodalix file manager.

mod browser;
mod columns;
mod customize;
mod dialogs;
mod external;
mod fileicons;
mod folders;
mod i18n;
mod icons;
mod items;
mod object;
mod properties;
mod share;
mod thumbnailers;
mod thumbs;
mod transfer;

use std::path::PathBuf;

use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::browser::Browser;

const APP_ID: &str = "os.nodalix.Archivador";
const STYLE: &str = include_str!("style.css");

/// Keyboard shortcuts, by action.
const ACCELS: &[(&str, &[&str])] = &[
    ("win.back", &["<Alt>Left", "BackSpace"]),
    ("win.forward", &["<Alt>Right"]),
    ("win.up", &["<Alt>Up"]),
    ("win.home", &["<Alt>Home"]),
    ("win.refresh", &["F5", "<Ctrl>r"]),
    ("win.new-folder", &["<Ctrl><Shift>n"]),
    ("win.copy", &["<Ctrl>c"]),
    ("win.cut", &["<Ctrl>x"]),
    ("win.paste", &["<Ctrl>v"]),
    ("win.select-all", &["<Ctrl>a"]),
    ("win.rename", &["F2"]),
    ("win.trash", &["Delete"]),
    ("win.delete", &["<Shift>Delete"]),
    ("win.open", &["<Ctrl>o"]),
    ("win.properties", &["<Alt>Return"]),
    ("win.search", &["<Ctrl>f"]),
    ("win.location", &["<Ctrl>l"]),
    ("win.show-hidden", &["<Ctrl>h"]),
    ("win.view::list", &["<Ctrl>1"]),
    ("win.view::grid", &["<Ctrl>2"]),
    ("win.view::columns", &["<Ctrl>3"]),
    ("win.close", &["<Ctrl>w"]),
    ("win.preferences", &["<Ctrl>comma"]),
];

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    // Started by D-Bus for a call, or left without windows, Archivador waits a little before it
    // quits, so the next "show in folder" does not have to start it again.
    app.set_inactivity_timeout(10_000);
    app.connect_startup(|app| {
        nodalix_ui::init(STYLE);
        // A drag should start as soon as the pointer means it, not after a long pull.
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_dnd_drag_threshold(4);
        }
        for (action, keys) in ACCELS {
            app.set_accels_for_action(action, keys);
        }
        external::serve(app);
    });
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            Browser::new(app, home()).present();
        }
    });
    // `nodalix-archivador /some/folder`, `--select file`, `--new-window`, `--quit`: the
    // command line Nautilus took.
    app.connect_command_line(external::command_line);
    // Opened through D-Bus activation of the app itself (a launcher with files): folders open,
    // a file opens its folder with the file selected.
    app.connect_open(|app, files, _| {
        for file in files {
            let Some(path) = file.path() else { continue };
            if path.is_dir() {
                Browser::new(app, path).present();
            } else if let Some(parent) = path.parent() {
                let browser = Browser::new(app, parent.to_path_buf());
                browser.select_after_load(vec![path]);
                browser.present();
            }
        }
    });
    app.run()
}

pub fn home() -> PathBuf {
    glib::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shortcut_is_a_valid_accelerator() {
        // Without a display (CI) there is nothing to parse with.
        if gtk::init().is_err() {
            return;
        }
        for (action, keys) in ACCELS {
            for key in *keys {
                assert!(
                    gtk::accelerator_parse(*key).is_some(),
                    "{key} for {action} does not parse"
                );
            }
        }
    }
}
