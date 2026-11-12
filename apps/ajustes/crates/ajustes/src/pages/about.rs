//! About: the system's identity and hardware.

use ajustes_core::sysinfo;
use monitor_core::format_bytes;
use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

use super::{column, scroller};
use crate::i18n::tr;

const LOGO: &str = "/usr/share/pixmaps/nodalix-logo.png";

fn row(title: &str, value: &str) -> ndl::ActionRow {
    let row = ndl::ActionRow::builder()
        .title(title)
        .subtitle(value)
        .build();
    // The value is something to copy: a long-press or select-all works on the label.
    row
}

pub fn build() -> gtk::ScrolledWindow {
    let info = sysinfo::read();
    let content = column();

    let hero = gtk::Box::new(gtk::Orientation::Vertical, 8);
    hero.set_halign(gtk::Align::Center);
    let logo = if std::path::Path::new(LOGO).is_file() {
        let picture = gtk::Picture::for_filename(LOGO);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::ScaleDown);
        picture.set_size_request(-1, 96);
        picture.upcast::<gtk::Widget>()
    } else {
        let image = gtk::Image::from_icon_name("help-about-symbolic");
        image.set_pixel_size(64);
        image.upcast::<gtk::Widget>()
    };
    logo.add_css_class("settings-logo");
    hero.append(&logo);
    let name = gtk::Label::new(Some(&info.os_name));
    name.add_css_class("settings-os-name");
    hero.append(&name);
    content.append(&hero);

    let device = ndl::PreferencesGroup::builder().title(tr("Device")).build();
    let unknown = tr("Unknown");
    let or_unknown = |v: &Option<String>| v.clone().unwrap_or_else(|| unknown.to_string());
    device.add(&row(
        tr("Device name"),
        if info.hostname.is_empty() {
            unknown
        } else {
            &info.hostname
        },
    ));
    device.add(&row(tr("Version"), &or_unknown(&info.nodalix_version)));
    device.add(&row(tr("Kernel"), &format!("Linux {}", info.kernel)));
    device.add(&row(tr("Architecture"), &info.arch));
    device.add(&row(tr("Processor"), &or_unknown(&info.cpu)));
    device.add(&row(
        tr("Memory"),
        &info.memory.map_or(unknown.to_string(), format_bytes),
    ));
    device.add(&row(
        tr("Disk"),
        &info.disk.map_or(unknown.to_string(), format_bytes),
    ));
    content.append(&device);

    scroller(&content)
}
