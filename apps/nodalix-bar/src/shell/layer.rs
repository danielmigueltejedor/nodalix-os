use gtk::prelude::*;

pub fn configure(window: &adw::ApplicationWindow) {
    window.set_title(Some("Nodalix Bar"));
    window.set_default_size(1180, 44);
    window.set_resizable(false);
    window.add_css_class("bar-window");

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
