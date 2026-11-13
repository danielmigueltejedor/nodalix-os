//! Nodalix Settings.

mod i18n;
mod pages;
mod search_provider;
mod window;

use nodalia_gtk::gtk::{self, gio, glib, prelude::*};

const APP_ID: &str = "os.nodalix.Settings";
const STYLE: &str = include_str!("style.css");

/// Writes a panic where it can be found afterwards: the standard error (the journal, when the
/// app starts from the launcher) and `~/.cache/nodalix/settings-crash.log`.
fn keep_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}\n{}\n", std::backtrace::Backtrace::force_capture());
        let folder = glib::user_cache_dir().join("nodalix");
        use std::io::Write;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        // Private to the user: a backtrace can name paths and values.
        if std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&folder)
            .is_ok()
        {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(folder.join("settings-crash.log"))
            {
                let _ = file.write_all(text.as_bytes());
            }
        }
        previous(info);
    }));
}

fn main() -> glib::ExitCode {
    keep_panics();
    // The Shell's search of Settings: a small process with no window and no GTK.
    if std::env::args().nth(1).as_deref() == Some("--search-provider") {
        return search_provider::run();
    }
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_startup(|app| {
        nodalix_ui::init(STYLE);
        // Settings decides the system's style: an old `default` becomes light.
        pages::appearance::settle_style();
        // `gapplication action os.nodalix.Settings open-page "'wifi'"`, and what a desktop
        // file's actions and `gio launch` use: the name of a page, or of a GNOME panel.
        let open = gio::SimpleAction::new("open-page", Some(glib::VariantTy::STRING));
        let handle = app.downgrade();
        open.connect_activate(move |_, name| {
            let Some(app) = handle.upgrade() else { return };
            let page = name
                .and_then(|name| name.str())
                .and_then(ajustes_core::pages::resolve);
            window::present(&app, page);
        });
        app.add_action(&open);
    });
    // `nodalix-settings wifi`, `nodalix-settings --page wifi`: the command line of
    // `gnome-control-center`, so what launches that keeps working. A name that is no page opens
    // the first one, without an error. A second launch is passed to the first one.
    app.connect_command_line(|app, line| {
        let args: Vec<String> = line
            .arguments()
            .iter()
            .skip(1)
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        window::present(app, ajustes_core::pages::from_args(&args));
        glib::ExitCode::SUCCESS
    });
    app.run()
}
