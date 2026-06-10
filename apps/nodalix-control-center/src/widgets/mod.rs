pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod media;
pub mod network;
pub mod power;
pub mod profile;
pub mod quick_actions;
pub mod toggles;

use gtk::prelude::*;

pub fn card(title: &str, icon: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.add_css_class("cc-card");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let icon_label = gtk::Label::new(Some(icon));
    icon_label.add_css_class("card-icon");
    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("card-title");
    title_label.set_xalign(0.0);
    title_label.set_hexpand(true);
    header.append(&icon_label);
    header.append(&title_label);
    card.append(&header);
    card
}

pub fn pill(label: &str, icon: &str, active: bool) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("cc-pill");
    if active {
        button.add_css_class("cc-pill-active");
    }

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon_label = gtk::Label::new(Some(icon));
    icon_label.add_css_class("pill-icon");
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    body.append(&icon_label);
    body.append(&text);
    button.set_child(Some(&body));
    button
}
