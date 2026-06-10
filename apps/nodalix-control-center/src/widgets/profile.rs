use gtk::prelude::*;
use nodalix_profile::{current_user, initials_from_name, load_profile};
use std::path::Path;

pub fn build() -> gtk::Box {
    let user = current_user();
    let profile = load_profile();

    let card = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    card.add_css_class("cc-profile");

    card.append(&avatar_widget(
        52,
        user.avatar.as_deref(),
        &initials_from_name(&user.display_name),
    ));

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_valign(gtk::Align::Center);
    text.set_hexpand(true);

    let name = gtk::Label::new(Some(&user.display_name));
    name.set_xalign(0.0);
    name.add_css_class("cc-profile-name");

    let username = gtk::Label::new(Some(&format!("@{}", user.username)));
    username.set_xalign(0.0);
    username.add_css_class("muted-label");

    let visual = gtk::Label::new(Some(&format!(
        "Tema {} · Acento {}",
        profile.visual.color_scheme, profile.visual.accent_preset
    )));
    visual.set_xalign(0.0);
    visual.add_css_class("muted-label");

    text.append(&name);
    text.append(&username);
    text.append(&visual);
    card.append(&text);

    let settings = gtk::Button::with_label("Ajustes");
    settings.add_css_class("quick-action");
    settings.connect_clicked(|_| {
        let _ = std::process::Command::new("nodalix-settings").spawn();
    });
    card.append(&settings);

    card
}

fn avatar_widget(size: i32, image: Option<&Path>, initials: &str) -> gtk::Widget {
    let clip = gtk::Box::new(gtk::Orientation::Vertical, 0);
    clip.add_css_class("cc-avatar-clip");
    clip.set_size_request(size, size);

    if let Some(path) = image.filter(|p| p.is_file()) {
        let picture = gtk::Picture::for_filename(path);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_size_request(size, size);
        clip.append(&picture);
    } else {
        let label = gtk::Label::new(Some(initials));
        label.set_vexpand(true);
        label.set_hexpand(true);
        label.set_valign(gtk::Align::Center);
        label.set_halign(gtk::Align::Center);
        label.add_css_class("cc-avatar-fallback");
        clip.append(&label);
    }

    clip.upcast()
}
