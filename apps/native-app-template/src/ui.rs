use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix App")
        .default_width(920)
        .default_height(620)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
    root.add_css_class("app-root");
    window.set_content(Some(&root));

    let title = gtk::Label::new(Some("Nodalix App"));
    title.add_css_class("app-title");
    title.set_xalign(0.0);
    root.append(&title);

    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("empty-card");
    let heading = gtk::Label::new(Some("Ready for a native Nodalix app"));
    heading.add_css_class("card-title");
    heading.set_xalign(0.0);
    let body = gtk::Label::new(Some(
        "Copy this template, rename the app id, and build from here.",
    ));
    body.add_css_class("muted-label");
    body.set_xalign(0.0);
    card.append(&heading);
    card.append(&body);
    root.append(&card);

    window.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/native-app-template.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
