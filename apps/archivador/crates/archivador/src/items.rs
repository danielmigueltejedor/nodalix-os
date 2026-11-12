//! How a file looks in the list and in the grid.

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use archivador_core::entry::Entry;
use archivador_core::format;
use ndl::gtk::{self, gdk, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::icons;
use crate::object::FileObject;
use crate::thumbs;
use crate::transfer;

thread_local! {
    static MODIFIED: gtk::SizeGroup = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
    static SIZE: gtk::SizeGroup = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
}

/// Makes the column header's cell as wide as the rows' modified-time cells.
pub fn align_modified(widget: &impl IsA<gtk::Widget>) {
    MODIFIED.with(|g| g.add_widget(widget));
}

/// Makes the column header's cell as wide as the rows' size cells.
pub fn align_size(widget: &impl IsA<gtk::Widget>) {
    SIZE.with(|g| g.add_widget(widget));
}

/// Called when a file is right-clicked: the widget, the position inside it, and the file's index.
pub type ContextMenu = Rc<dyn Fn(&gtk::Widget, f64, f64, u32)>;

/// What a file's row or tile needs from the window.
#[derive(Clone)]
pub struct Hooks {
    pub menu: ContextMenu,
    /// The paths a drag that starts on this file carries: all that are selected, if it is one of
    /// them, else just it.
    pub drag_paths: Rc<dyn Fn(u32) -> Vec<std::path::PathBuf>>,
    /// Files dropped on the folder at this position; whether they are copied. True if taken.
    pub drop_into: Rc<dyn Fn(u32, Vec<std::path::PathBuf>, bool) -> bool>,
}

/// Lets the file be dragged out, and a folder take what is dropped on it.
fn attach_drag_and_drop(item: &gtk::ListItem, widget: &gtk::Widget, hooks: &Hooks) {
    let source = gtk::DragSource::new();
    source.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);
    let (position, drag_paths) = (item.clone(), hooks.drag_paths.clone());
    source.connect_prepare(move |_, _, _| {
        let paths = drag_paths(position.position());
        (!paths.is_empty()).then(|| transfer::drag_content(&paths))
    });
    source.connect_drag_begin(|source, _| {
        if let Some(widget) = source.widget() {
            source.set_icon(Some(&gtk::WidgetPaintable::new(Some(&widget))), 0, 0);
        }
    });
    widget.add_controller(source);

    let target = gtk::DropTarget::new(
        gdk::FileList::static_type(),
        gdk::DragAction::COPY | gdk::DragAction::MOVE,
    );
    let folder = item.clone();
    target.connect_accept(move |_, _| object(&folder).is_some_and(|f| f.entry().is_dir));
    let (position, drop_into) = (item.clone(), hooks.drop_into.clone());
    target.connect_drop(move |target, value, _, _| {
        drop_into(
            position.position(),
            transfer::paths_of(value),
            transfer::drop_copies(target),
        )
    });
    widget.add_controller(target);
}

/// `14:05` today, `3 Nov` this year, `3 Nov 2025` before.
pub fn modified_text(time: Option<SystemTime>) -> String {
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

/// Shows the file's icon, and its thumbnail in place of it once that is ready. The image's name
/// is the file it was last asked to show, so a row that was recycled meanwhile is left alone.
pub fn show_icon(image: &gtk::Image, entry: &Entry, thumb_size: i32) {
    let path = entry.path.to_string_lossy().into_owned();
    image.set_widget_name(&path);
    // Rounded corners only clip what is drawn inside when the widget says so.
    image.set_overflow(gtk::Overflow::Hidden);
    image.remove_css_class("archivador-thumb");
    if let Some(texture) = thumbs::cached(entry, thumb_size) {
        image.set_paintable(Some(&texture));
        image.add_css_class("archivador-thumb");
        return;
    }
    icons::show(image, entry);
    if thumbs::wanted(entry) {
        let image = image.downgrade();
        thumbs::load(entry, thumb_size, move |texture| {
            if let Some(image) = image.upgrade() {
                if image.widget_name() == path {
                    image.set_paintable(Some(&texture));
                    image.add_css_class("archivador-thumb");
                }
            }
        });
    }
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
pub fn list_factory(hooks: Hooks) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("archivador-row");
        let icon = gtk::Image::new();
        icon.set_pixel_size(32);
        let name = gtk::Label::new(None);
        name.set_hexpand(true);
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let modified = gtk::Label::new(None);
        modified.add_css_class("archivador-meta");
        modified.set_width_chars(11);
        modified.set_xalign(1.0);
        align_modified(&modified);
        let size = gtk::Label::new(None);
        size.add_css_class("archivador-meta");
        size.set_width_chars(10);
        size.set_xalign(1.0);
        align_size(&size);
        for widget in [
            icon.upcast_ref::<gtk::Widget>(),
            name.upcast_ref(),
            modified.upcast_ref(),
            size.upcast_ref(),
        ] {
            row.append(widget);
        }
        on_right_click(item, row.upcast_ref(), &hooks.menu);
        attach_drag_and_drop(item, row.upcast_ref(), &hooks);
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
            show_icon(&icon, &entry, 96);
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
pub fn grid_factory(hooks: Hooks) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let tile = gtk::Box::new(gtk::Orientation::Vertical, 6);
        tile.add_css_class("archivador-tile");
        tile.set_halign(gtk::Align::Center);
        let icon = gtk::Image::new();
        icon.set_pixel_size(80);
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
        on_right_click(item, tile.upcast_ref(), &hooks.menu);
        attach_drag_and_drop(item, tile.upcast_ref(), &hooks);
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
            show_icon(&icon, &entry, 256);
        }
        if let Some(name) = tile.last_child().and_downcast::<gtk::Label>() {
            name.set_label(&entry.name);
            name.set_tooltip_text(Some(&entry.name));
        }
    });
    factory
}

/// A narrow row for the columns view: a small icon, the name, and an arrow on folders. Rows in
/// the columns that only show a folder on the way take no hooks.
pub fn column_factory(hooks: Option<Hooks>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("archivador-row");
        let icon = gtk::Image::new();
        icon.set_pixel_size(24);
        let name = gtk::Label::new(None);
        name.set_hexpand(true);
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let arrow = gtk::Image::from_icon_name("go-next-symbolic");
        arrow.add_css_class("archivador-arrow");
        row.append(&icon);
        row.append(&name);
        row.append(&arrow);
        if let Some(hooks) = &hooks {
            on_right_click(item, row.upcast_ref(), &hooks.menu);
            attach_drag_and_drop(item, row.upcast_ref(), hooks);
        }
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
        if let Some(icon) = row.first_child().and_downcast::<gtk::Image>() {
            show_icon(&icon, &entry, 96);
        }
        if let Some(name) = row.first_child().and_then(|w| w.next_sibling()) {
            if let Some(name) = name.downcast_ref::<gtk::Label>() {
                name.set_label(&entry.name);
            }
        }
        if let Some(arrow) = row.last_child() {
            arrow.set_visible(entry.is_dir);
        }
    });
    factory
}
