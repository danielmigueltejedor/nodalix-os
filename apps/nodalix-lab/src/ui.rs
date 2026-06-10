use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

const SECTIONS: &[&str] = &[
    "Calculator",
    "Units",
    "Equations",
    "Matrices",
    "Atmosphere",
    "Aerodynamics",
    "Structures",
    "Plots",
];

pub fn build(app: &adw::Application) {
    load_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Lab")
        .default_width(1080)
        .default_height(700)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("lab-root");
    window.set_content(Some(&root));
    root.append(&sidebar());
    root.append(&content());
    window.present();
}

fn sidebar() -> gtk::Box {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 10);
    sidebar.add_css_class("lab-sidebar");
    sidebar.set_size_request(230, -1);
    let title = gtk::Label::new(Some("Nodalix Lab"));
    title.add_css_class("app-title");
    title.set_xalign(0.0);
    sidebar.append(&title);
    for section in SECTIONS {
        let button = gtk::Button::with_label(section);
        button.add_css_class("nav-button");
        sidebar.append(&button);
    }
    sidebar
}

fn content() -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.add_css_class("lab-content");
    content.set_hexpand(true);
    let title = gtk::Label::new(Some("Engineering toolbox"));
    title.add_css_class("page-title");
    title.set_xalign(0.0);
    content.append(&title);
    for text in [
        "Scientific calculator and equations",
        "Unit conversion and constants",
        "Matrices, interpolation, integration, derivatives",
        "Atmosphere, aerodynamics, structures, and quick plots",
    ] {
        content.append(&card(text));
    }
    content
}

fn card(text: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("lab-card");
    let label = gtk::Label::new(Some(text));
    label.add_css_class("card-title");
    label.set_xalign(0.0);
    card.append(&label);
    card
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-lab.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
