use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Control Center")
        .build();
    crate::shell::window::configure(&window);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.add_css_class("control-center-root");
    window.set_content(Some(&root));

    let title = gtk::Label::new(Some("Nodalix Control Center"));
    title.add_css_class("panel-title");
    title.set_xalign(0.0);
    root.append(&title);

    root.append(&crate::widgets::network::build());
    root.append(&crate::widgets::audio::build());
    root.append(&crate::widgets::brightness::build());
    root.append(&crate::widgets::media::build());
    root.append(&crate::widgets::toggles::build());
    root.append(&crate::widgets::power::build(&window));
    root.append(&crate::widgets::quick_actions::build());

    window.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/nodalix-control-center.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
