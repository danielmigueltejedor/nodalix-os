use gtk::prelude::*;

pub fn run() {
    let app = adw::Application::builder()
        .application_id("com.nodalia.lix.cad")
        .build();

    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
    });
    app.connect_activate(crate::ui::build);
    app.run();
}
