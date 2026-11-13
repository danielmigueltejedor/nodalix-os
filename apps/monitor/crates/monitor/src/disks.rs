//! Mounted file systems.

use std::cell::Cell;
use std::rc::Rc;

use monitor_core::format_bytes;
use monitor_core::fs::{self, Kind, Mount};
use nodalia_gtk::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk::ActionRow;

use crate::i18n::tr;

pub struct DiskView {
    pub widget: gtk::ScrolledWindow,
    list: gtk::ListBox,
    /// A read is in flight: a hung network mount can make `statvfs` wait for good, so reads run
    /// on a worker thread, one at a time, and the window never waits for them.
    reading: Cell<bool>,
}

impl DiskView {
    pub fn new() -> Rc<Self> {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        let clamp = gtk::Box::new(gtk::Orientation::Vertical, 0);
        clamp.set_margin_start(8);
        clamp.set_margin_end(8);
        clamp.set_margin_bottom(12);
        clamp.append(&list);
        let widget = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .vexpand(true)
            .build();
        let view = Rc::new(Self {
            widget,
            list,
            reading: Cell::new(false),
        });
        view.update();
        view
    }

    fn row(mount: &Mount) -> ActionRow {
        let row = ActionRow::builder()
            .title(mount.mount_point.clone())
            .subtitle(format!("{} · {}", mount.device, mount.fs_type))
            .build();
        let icon = gtk::Image::from_icon_name(match mount.kind() {
            Kind::Disk => "drive-harddisk-symbolic",
            Kind::Removable => "drive-removable-media-symbolic",
            Kind::Network => "folder-remote-symbolic",
        });
        icon.set_pixel_size(20);
        row.add_prefix(&icon);
        let side = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        if mount.has_size() {
            let bar = gtk::ProgressBar::new();
            bar.set_fraction(mount.fraction());
            bar.set_valign(gtk::Align::Center);
            bar.set_size_request(160, -1);
            let text = gtk::Label::new(Some(&format!(
                "{} {} · {} {}",
                tr("Used"),
                format_bytes(mount.used()),
                tr("Free"),
                format_bytes(mount.free)
            )));
            text.add_css_class("dim-label");
            text.set_valign(gtk::Align::Center);
            side.append(&text);
            side.append(&bar);
        } else {
            // No limit is known: say what is used and leave the bar out.
            let text = gtk::Label::new(Some(&format!(
                "{} {} · {}",
                tr("Used"),
                format_bytes(mount.used()),
                tr("No size limit")
            )));
            text.add_css_class("dim-label");
            side.append(&text);
        }
        row.add_suffix(&side);
        row
    }

    /// Disks change rarely; the caller refreshes this less often than the other views. The
    /// list keeps showing the last reading until the new one arrives.
    pub fn update(self: &Rc<Self>) {
        if self.reading.replace(true) {
            return;
        }
        let view = self.clone();
        glib::spawn_future_local(async move {
            let mounts = gio::spawn_blocking(fs::read).await.unwrap_or_default();
            view.reading.set(false);
            view.show(&mounts);
        });
    }

    fn show(&self, mounts: &[Mount]) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        if mounts.is_empty() {
            let empty = gtk::Label::new(Some(tr("No mounted file systems")));
            empty.add_css_class("dim-label");
            empty.set_margin_top(24);
            empty.set_margin_bottom(24);
            self.list.append(&empty);
        }
        for m in mounts {
            self.list.append(&Self::row(m));
        }
    }
}
