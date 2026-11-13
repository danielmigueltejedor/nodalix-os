//! Nodalix Settings.

mod i18n;
mod legacy;
mod pages;
mod window;

use nodalia_gtk::gtk::{self, glib, prelude::*};

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
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| nodalix_ui::init(STYLE));
    app.connect_activate(window::present);
    app.run()
}
