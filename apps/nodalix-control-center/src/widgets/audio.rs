use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Audio", "󰕾");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.append(&gtk::Label::new(Some("󰕾")));
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_hexpand(true);
    scale.set_value(50.0);
    row.append(&scale);
    let mute = gtk::Button::with_label("󰖁");
    mute.add_css_class("icon-button");
    row.append(&mute);
    card.append(&row);

    let device = gtk::Label::new(Some(&format!("Salida: {}", crate::system::volume_label())));
    device.add_css_class("muted-label");
    device.set_xalign(0.0);
    card.append(&device);
    card
}
