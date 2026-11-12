//! Archivador, the Nodalix file manager.

mod browser;
mod dialogs;
mod i18n;
mod icons;
mod items;
mod object;
mod properties;
mod share;
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
    ("win.close", &["<Ctrl>w"]),
];

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_startup(|app| {
        nodalix_ui::init(STYLE);
        for (action, keys) in ACCELS {
            app.set_accels_for_action(action, keys);
        }
    });
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            Browser::new(app, home()).present();
        }
    });
    // `nodalix-archivador /some/folder`, or a file: its folder opens with the file selected.
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
