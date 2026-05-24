use gtk::prelude::*;

pub fn configure(window: &adw::ApplicationWindow) {
    window.set_title(Some("Nodalix Control Center"));
    window.set_default_size(430, 620);
    window.set_resizable(false);
    window.add_css_class("control-center-window");

    let controller = gtk::EventControllerKey::new();
    let close_window = window.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            close_window.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.add_controller(controller);
}
