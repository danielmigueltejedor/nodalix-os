use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};
use std::path::{Path, PathBuf};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Bar")
        .build();
    crate::shell::layer::configure(&window);

    let root = gtk::CenterBox::new();
    root.add_css_class("bar-root");
    window.set_content(Some(&root));

    let left = island();
    let logo = gtk::Button::new();
    logo.add_css_class("bar-button");
    logo.set_child(Some(&brand_content("Nodalix", 20)));
    logo.connect_clicked(|_| {
        crate::modules::spawn_detached("nodalix-control-center", &[]);
    });
    left.append(&logo);
    root.set_start_widget(Some(&left));

    let center = island();
    center.add_css_class("center-island");
    let workspace = gtk::Label::new(Some(&crate::modules::workspaces::label()));
    workspace.add_css_class("workspace-label");
    let title = gtk::Label::new(Some(&crate::modules::window_title::label()));
    title.add_css_class("window-title");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    center.append(&workspace);
    center.append(&title);
    root.set_center_widget(Some(&center));

    let right = island();
    for text in [
        crate::modules::network::label(),
        crate::modules::bluetooth::label(),
        crate::modules::audio::label(),
        crate::modules::updates::label(),
        crate::modules::system_tray::label().to_string(),
        crate::modules::clock::label(),
        crate::modules::power::label().to_string(),
    ] {
        let label = gtk::Label::new(Some(&text));
        label.add_css_class("status-pill");
        right.append(&label);
    }
    let control = gtk::Button::with_label("󰕮");
    control.add_css_class("bar-button");
    control.connect_clicked(|_| {
        crate::modules::spawn_detached("nodalix-control-center", &[]);
    });
    right.append(&control);
    root.set_end_widget(Some(&right));

    window.present();
}

fn brand_content(text: &str, size: i32) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_valign(gtk::Align::Center);
    if let Some(path) = resolve_logo_path() {
        let image = gtk::Picture::for_filename(path);
        image.set_content_fit(gtk::ContentFit::Contain);
        image.set_size_request(size, size);
        image.add_css_class("brand-logo");
        content.append(&image);
    }
    let label = gtk::Label::new(Some(text));
    label.add_css_class("brand-label");
    content.append(&label);
    content
}

fn resolve_logo_path() -> Option<PathBuf> {
    [
        "/etc/nodalix/brand/nodalix-logo-symbol.svg",
        "/usr/share/nodalix/brand/nodalix-logo-symbol.svg",
        "/home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg",
    ]
    .iter()
    .map(Path::new)
    .find(|path| path.is_file())
    .map(Path::to_path_buf)
}

fn island() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    box_.add_css_class("bar-island");
    box_
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-bar.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
