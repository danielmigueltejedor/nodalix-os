//! How a file looks in the list and in the grid.

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use archivador_core::entry::Entry;
use archivador_core::format;
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::object::FileObject;

/// Called when a file is right-clicked: the widget, the position inside it, and the file's index.
pub type ContextMenu = Rc<dyn Fn(&gtk::Widget, f64, f64, u32)>;

fn icon_for(entry: &Entry) -> gio::Icon {
    if entry.is_dir {
        return gio::ThemedIcon::new("folder").upcast();
    }
    let (content_type, _) = gio::content_type_guess(Some(&entry.name), None::<&[u8]>);
    gio::content_type_get_icon(&content_type)
}

/// `14:05` today, `3 Nov` this year, `3 Nov 2025` before.
fn modified_text(time: Option<SystemTime>) -> String {
    let Some(secs) = time
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
    else {
        return String::new();
    };
    let (Ok(date), Ok(now)) = (
        glib::DateTime::from_unix_local(secs),
        glib::DateTime::now_local(),
    ) else {
        return String::new();
    };
    let layout = if date.year() == now.year() && date.day_of_year() == now.day_of_year() {
        "%H:%M"
    } else if date.year() == now.year() {
        "%e %b"
    } else {
        "%e %b %Y"
    };
    date.format(layout)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn on_right_click(item: &gtk::ListItem, widget: &gtk::Widget, menu: &ContextMenu) {
    let click = gtk::GestureClick::builder().button(3).build();
    let (item, menu, target) = (item.clone(), menu.clone(), widget.clone());
    click.connect_pressed(move |gesture, _, x, y| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        menu(target.upcast_ref(), x, y, item.position());
    });
    widget.add_controller(click);
}

fn object(item: &gtk::ListItem) -> Option<FileObject> {
    item.item().and_downcast::<FileObject>()
}

/// A row per file: icon, name, modification time, size.
pub fn list_factory(menu: ContextMenu) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("archivador-row");
        let icon = gtk::Image::new();
        icon.set_pixel_size(24);
        let name = gtk::Label::new(None);
        name.set_hexpand(true);
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let modified = gtk::Label::new(None);
        modified.add_css_class("archivador-meta");
        modified.set_width_chars(11);
        modified.set_xalign(1.0);
        let size = gtk::Label::new(None);
        size.add_css_class("archivador-meta");
        size.set_width_chars(10);
        size.set_xalign(1.0);
        for widget in [
            icon.upcast_ref::<gtk::Widget>(),
            name.upcast_ref(),
            modified.upcast_ref(),
            size.upcast_ref(),
        ] {
            row.append(widget);
        }
        on_right_click(item, row.upcast_ref(), &menu);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let (Some(file), Some(row)) = (object(item), item.child()) else {
            return;
        };
        let entry = file.entry();
        let mut cells = std::iter::successors(row.first_child(), |w| w.next_sibling());
        if let Some(icon) = cells.next().and_downcast::<gtk::Image>() {
            icon.set_from_gicon(&icon_for(&entry));
        }
        if let Some(name) = cells.next().and_downcast::<gtk::Label>() {
            name.set_label(&entry.name);
        }
        if let Some(modified) = cells.next().and_downcast::<gtk::Label>() {
            modified.set_label(&modified_text(entry.modified));
        }
        if let Some(size) = cells.next().and_downcast::<gtk::Label>() {
            size.set_label(&if entry.is_dir {
                String::new()
            } else {
                format::size(entry.size)
            });
        }
    });
    factory
}

/// A tile per file: a large icon above the name.
pub fn grid_factory(menu: ContextMenu) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let tile = gtk::Box::new(gtk::Orientation::Vertical, 6);
        tile.add_css_class("archivador-tile");
        tile.set_halign(gtk::Align::Center);
        let icon = gtk::Image::new();
        icon.set_pixel_size(64);
        let name = gtk::Label::new(None);
        name.set_wrap(true);
        name.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        name.set_lines(2);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.set_max_width_chars(14);
        name.set_width_chars(14);
        name.set_justify(gtk::Justification::Center);
        tile.append(&icon);
        tile.append(&name);
        on_right_click(item, tile.upcast_ref(), &menu);
        item.set_child(Some(&tile));
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let (Some(file), Some(tile)) = (object(item), item.child()) else {
            return;
        };
        let entry = file.entry();
        if let Some(icon) = tile.first_child().and_downcast::<gtk::Image>() {
            icon.set_from_gicon(&icon_for(&entry));
        }
        if let Some(name) = tile.last_child().and_downcast::<gtk::Label>() {
            name.set_label(&entry.name);
            name.set_tooltip_text(Some(&entry.name));
        }
    });
    factory
}
