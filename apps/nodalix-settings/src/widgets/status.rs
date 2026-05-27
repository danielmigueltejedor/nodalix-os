use gtk::prelude::*;

#[derive(Clone)]
pub struct StatusStrip {
    pub root: gtk::Box,
    spinner: gtk::Spinner,
    icon: gtk::Label,
    message: gtk::Label,
}

impl StatusStrip {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        root.add_css_class("status-strip");
        root.set_halign(gtk::Align::Fill);
        root.set_visible(false);

        let spinner = gtk::Spinner::new();
        spinner.add_css_class("status-spinner");
        spinner.set_visible(false);

        let icon = gtk::Label::new(None);
        icon.add_css_class("status-icon");
        icon.set_visible(false);

        let message = gtk::Label::new(None);
        message.set_xalign(0.0);
        message.set_hexpand(true);
        message.set_wrap(true);
        message.add_css_class("status-message");

        root.append(&spinner);
        root.append(&icon);
        root.append(&message);

        Self {
            root,
            spinner,
            icon,
            message,
        }
    }

    pub fn set_loading(&self, text: &str) {
        self.root.set_visible(true);
        self.root.remove_css_class("status-success");
        self.root.remove_css_class("status-error");
        self.spinner.set_visible(true);
        self.spinner.start();
        self.icon.set_visible(false);
        self.message.set_text(text);
    }

    pub fn set_success(&self, text: &str) {
        self.root.set_visible(true);
        self.root.remove_css_class("status-error");
        self.root.add_css_class("status-success");
        self.spinner.stop();
        self.spinner.set_visible(false);
        self.icon.set_text("✓");
        self.icon.set_visible(true);
        self.message.set_text(text);
    }

    pub fn set_error(&self, text: &str) {
        self.root.set_visible(true);
        self.root.remove_css_class("status-success");
        self.root.add_css_class("status-error");
        self.spinner.stop();
        self.spinner.set_visible(false);
        self.icon.set_text("!");
        self.icon.set_visible(true);
        self.message.set_text(text);
    }

}
