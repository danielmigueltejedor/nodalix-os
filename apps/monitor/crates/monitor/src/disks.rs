//! Mounted file systems.

use monitor_core::format_bytes;
use monitor_core::fs::{self, Mount};
use nodalia_gtk::gtk::{self, prelude::*};
use nodalia_gtk::ActionRow;

use crate::i18n::tr;

pub struct DiskView {
    pub widget: gtk::ScrolledWindow,
    list: gtk::ListBox,
}

impl DiskView {
    pub fn new() -> Self {
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
        let view = Self { widget, list };
        view.update();
        view
    }

    fn row(mount: &Mount) -> ActionRow {
        let row = ActionRow::builder()
            .title(mount.mount_point.clone())
            .subtitle(format!("{} · {}", mount.device, mount.fs_type))
            .build();
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
        let side = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        side.append(&text);
        side.append(&bar);
        row.add_suffix(&side);
        row
    }

    /// Disks change rarely; the caller refreshes this less often than the other views.
    pub fn update(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let mounts = fs::read();
        if mounts.is_empty() {
            let empty = gtk::Label::new(Some(tr("No mounted file systems")));
            empty.add_css_class("dim-label");
            empty.set_margin_top(24);
            empty.set_margin_bottom(24);
            self.list.append(&empty);
        }
        for m in &mounts {
            self.list.append(&Self::row(m));
        }
    }
}
