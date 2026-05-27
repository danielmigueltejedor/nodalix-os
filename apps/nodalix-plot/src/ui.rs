use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Plot")
        .default_width(980)
        .default_height(640)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    root.add_css_class("plot-root");
    window.set_content(Some(&root));
    let title = gtk::Label::new(Some("Nodalix Plot"));
    title.add_css_class("app-title");
    title.set_xalign(0.0);
    root.append(&title);
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for action in ["Import CSV", "Function", "Log scale", "Export"] {
        let button = gtk::Button::with_label(action);
        button.add_css_class("plot-button");
        toolbar.append(&button);
    }
    root.append(&toolbar);
    let canvas = gtk::Label::new(Some("2D plot canvas placeholder"));
    canvas.add_css_class("plot-canvas");
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    root.append(&canvas);
    window.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-plot.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
