use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Dock")
        .build();
    crate::shell::layer::configure(&window);

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    root.add_css_class("dock-root");
    window.set_content(Some(&root));

    let running = crate::windows::running_classes();
    for pinned in crate::apps::pinned() {
        let item = dock_item(
            &pinned,
            crate::windows::is_running(&running, pinned.class_hint),
        );
        root.append(&item);
    }

    window.present();
}

fn dock_item(app: &crate::apps::PinnedApp, running: bool) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("dock-item");
    if running {
        button.add_css_class("dock-item-running");
    }
    button.set_tooltip_text(Some(app.name));

    let body = gtk::Box::new(gtk::Orientation::Vertical, 3);
    body.set_halign(gtk::Align::Center);
    let icon = gtk::Label::new(Some(app.icon));
    icon.add_css_class("dock-icon");
    let dot = gtk::Label::new(Some(if running { "●" } else { " " }));
    dot.add_css_class("running-dot");
    body.append(&icon);
    body.append(&dot);
    button.set_child(Some(&body));

    let command = app.command.to_string();
    button.connect_clicked(move |_| crate::apps::launch(&command));
    button
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-dock.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
