use gtk::prelude::*;
use std::path::Path;

pub fn avatar_widget(size: i32, image: Option<&Path>, initials: &str) -> gtk::Widget {
    let clip = gtk::Box::new(gtk::Orientation::Vertical, 0);
    clip.add_css_class("avatar-clip");
    clip.set_size_request(size, size);
    clip.set_halign(gtk::Align::Start);
    clip.set_valign(gtk::Align::Start);

    if let Some(path) = image.filter(|p| p.is_file()) {
        let picture = gtk::Picture::for_filename(path);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_size_request(size, size);
        picture.add_css_class("avatar-image");
        clip.append(&picture);
    } else {
        let label = gtk::Label::new(Some(initials));
        label.set_vexpand(true);
        label.set_hexpand(true);
        label.set_valign(gtk::Align::Center);
        label.set_halign(gtk::Align::Center);
        label.add_css_class("avatar-fallback");
        clip.append(&label);
    }

    clip.upcast()
}

pub fn initials_from_name(name: &str) -> String {
    let result = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if result.is_empty() {
        "?".to_string()
    } else {
        result.to_uppercase()
    }
}
