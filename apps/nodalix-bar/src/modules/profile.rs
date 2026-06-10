use gtk::prelude::*;
use nodalix_profile::{current_user, initials_from_name};
use std::path::Path;

pub fn avatar_button(size: i32) -> gtk::Button {
    let user = current_user();
    let button = gtk::Button::new();
    button.add_css_class("bar-button");
    button.add_css_class("bar-avatar-button");
    button.set_child(Some(&avatar_content(
        size,
        user.avatar.as_deref(),
        &initials_from_name(&user.display_name),
    )));
    button.set_tooltip_text(Some(&format!("{} (@{})", user.display_name, user.username)));
    button.connect_clicked(|_| {
        crate::modules::spawn_detached("nodalix-settings", &[]);
    });
    button
}

fn avatar_content(size: i32, image: Option<&Path>, initials: &str) -> gtk::Widget {
    let clip = gtk::Box::new(gtk::Orientation::Vertical, 0);
    clip.add_css_class("bar-avatar-clip");
    clip.set_size_request(size, size);

    if let Some(path) = image.filter(|p| p.is_file()) {
        let picture = gtk::Picture::for_filename(path);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_size_request(size, size);
        picture.add_css_class("bar-avatar-image");
        clip.append(&picture);
    } else {
        let label = gtk::Label::new(Some(initials));
        label.set_vexpand(true);
        label.set_hexpand(true);
        label.set_valign(gtk::Align::Center);
        label.set_halign(gtk::Align::Center);
        label.add_css_class("bar-avatar-fallback");
        clip.append(&label);
    }

    clip.upcast()
}
