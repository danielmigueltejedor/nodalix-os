use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Multimedia", "󰝚");
    let track = gtk::Label::new(Some(&crate::system::media_label()));
    track.add_css_class("media-title");
    track.set_xalign(0.0);
    track.set_wrap(true);
    card.append(&track);

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (icon, command) in [("󰒮", "previous"), ("󰐊", "play-pause"), ("󰒭", "next")] {
        let button = gtk::Button::with_label(icon);
        button.add_css_class("icon-button");
        button.connect_clicked(move |_| {
            if crate::system::command_exists("playerctl") {
                let _ = crate::system::run_command("playerctl", &[command]);
            }
        });
        controls.append(&button);
    }
    card.append(&controls);
    card
}
