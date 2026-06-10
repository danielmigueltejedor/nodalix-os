use gtk::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub const BAR_HEIGHT: i32 = 38;

pub fn configure(window: &adw::ApplicationWindow, monitor: Option<&gtk::gdk::Monitor>) {
    let width = monitor
        .map(|monitor| monitor.geometry().width())
        .filter(|width| *width > 0)
        .unwrap_or(1920);

    window.set_title(Some("Nodalix Bar"));
    window.set_default_size(width, BAR_HEIGHT);
    window.set_size_request(width, BAR_HEIGHT);
    window.set_resizable(false);
    window.set_decorated(false);
    window.add_css_class("bar-window");

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some("nodalix-bar"));
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, 0);
    window.set_margin(Edge::Left, 0);
    window.set_margin(Edge::Right, 0);
    window.set_exclusive_zone(BAR_HEIGHT);
    if let Some(monitor) = monitor {
        window.set_monitor(Some(monitor));
    }

    let controller = gtk::EventControllerKey::new();
    let close_window = window.clone();
    controller.connect_key_pressed(move |_, key, _, modifier| {
        if key == gtk::gdk::Key::Escape
            || (key == gtk::gdk::Key::q && modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK))
        {
            close_window.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.add_controller(controller);
}
