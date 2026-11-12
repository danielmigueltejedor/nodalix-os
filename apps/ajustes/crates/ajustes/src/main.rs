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
        if std::fs::create_dir_all(&folder).is_ok() {
            let _ = std::fs::write(folder.join("settings-crash.log"), &text);
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
