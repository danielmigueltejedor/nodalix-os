//! Nodalix Settings.

mod i18n;
mod legacy;
mod pages;
mod window;

use nodalia_gtk::gtk::{self, glib, prelude::*};

const APP_ID: &str = "os.nodalix.Settings";
const STYLE: &str = include_str!("style.css");

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| nodalix_ui::init(STYLE));
    app.connect_activate(window::present);
    app.run()
}
