use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Brillo", "󰃠");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.append(&gtk::Label::new(Some("󰃠")));
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_hexpand(true);
    scale.set_value(65.0);
    if !crate::system::command_exists("brightnessctl") {
        scale.set_sensitive(false);
    }
    row.append(&scale);
    card.append(&row);
    let label = gtk::Label::new(Some(&crate::system::brightness_label()));
    label.add_css_class("muted-label");
    label.set_xalign(0.0);
    card.append(&label);
    card
}
