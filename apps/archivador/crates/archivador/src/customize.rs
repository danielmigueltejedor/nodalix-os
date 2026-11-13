//! Giving a folder a look of its own, as macOS lets one: a colour from a small palette, or a
//! picture of the user's choosing. Both are kept in the folder's own metadata, so they follow it
//! wherever Archivador (or any file manager that reads `metadata::custom-icon`) shows it.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::folders::{self, Color, Kind};
use crate::i18n::tr;
use crate::icons::{self, FOLDER_COLOR};

const CUSTOM_ICON: &str = "metadata::custom-icon";

/// Writes one metadata attribute. An empty value is how a look is taken off again: the readers
/// treat it as none.
fn set(path: &Path, attribute: &str, value: &str) -> Result<(), glib::Error> {
    gio::File::for_path(path).set_attribute_string(
        attribute,
        value,
        gio::FileQueryInfoFlags::NONE,
        gio::Cancellable::NONE,
    )?;
    Ok(())
}

/// Gives the folder this colour, and drops any picture of its own it had.
pub fn set_color(path: &Path, color: Color) -> Result<(), glib::Error> {
    set(path, FOLDER_COLOR, color.id())?;
    set(path, CUSTOM_ICON, "")
}

/// Gives the folder this picture in place of its icon.
pub fn set_picture(path: &Path, picture: &Path) -> Result<(), glib::Error> {
    set(path, CUSTOM_ICON, &picture.to_string_lossy())
}

/// Takes the folder's colour and picture off again.
pub fn reset(path: &Path) -> Result<(), glib::Error> {
    set(path, FOLDER_COLOR, "")?;
    set(path, CUSTOM_ICON, "")
}

/// Opens the dialog for the folder at `path`. `changed` runs after each change, so the window can
/// draw the folder anew.
pub fn present(parent: &impl IsA<gtk::Widget>, path: PathBuf, changed: impl Fn() + 'static) {
    let changed: Rc<dyn Fn()> = Rc::new(changed);
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let entry = archivador_core::entry::Entry {
        name: name.clone(),
        path: path.clone(),
        is_dir: true,
        ..Default::default()
    };

    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.add_css_class("archivador-properties");

    let preview = gtk::Image::new();
    preview.set_pixel_size(96);
    preview.set_halign(gtk::Align::Center);
    let title = gtk::Label::new(Some(&name));
    title.add_css_class("title-3");
    title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    title.set_max_width_chars(28);
    body.append(&preview);
    body.append(&title);

    let colors = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    colors.set_halign(gtk::Align::Center);
    let swatches: Rc<RefCell<Vec<(Color, gtk::Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let error = gtk::Label::new(None);
    error.add_css_class("dim-label");
    error.set_wrap(true);
    error.set_visible(false);

    // What the dialog shows follows what the folder has now.
    let refresh = {
        let preview = preview.clone();
        let swatches = swatches.clone();
        let entry = entry.clone();
        Rc::new(move || {
            icons::show(&preview, &entry);
            let current = icons::own_color(&entry).unwrap_or(Color::Teal);
            for (color, button) in swatches.borrow().iter() {
                if current == *color {
                    button.add_css_class("archivador-swatch-current");
                } else {
                    button.remove_css_class("archivador-swatch-current");
                }
            }
        })
    };
    let apply = {
        let changed = changed.clone();
        let refresh = refresh.clone();
        let error = error.clone();
        Rc::new(move |result: Result<(), glib::Error>| match result {
            Ok(()) => {
                error.set_visible(false);
                refresh();
                changed();
            }
            Err(e) => {
                error.set_label(&e.to_string());
                error.set_visible(true);
            }
        })
    };

    for color in Color::ALL {
        let button = gtk::Button::new();
        button.add_css_class("flat");
        button.add_css_class("archivador-swatch");
        button.set_tooltip_text(Some(tr(color.title())));
        let image = gtk::Image::new();
        image.set_pixel_size(40);
        icons::show_icon(&image, &folders::icon(Kind::Plain, color));
        button.set_child(Some(&image));
        let apply = apply.clone();
        let path = path.clone();
        button.connect_clicked(move |_| apply(set_color(&path, color)));
        colors.append(&button);
        swatches.borrow_mut().push((color, button));
    }
    body.append(&colors);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::Center);
    let choose = gtk::Button::with_label(tr("Choose image…"));
    let clear = gtk::Button::with_label(tr("Reset"));
    let close = gtk::Button::with_label(tr("Close"));
    close.add_css_class("suggested-action");
    actions.append(&choose);
    actions.append(&clear);
    actions.append(&close);
    body.append(&error);
    body.append(&actions);

    {
        let apply = apply.clone();
        let path = path.clone();
        clear.connect_clicked(move |_| apply(reset(&path)));
    }
    {
        let apply = apply.clone();
        let path = path.clone();
        let anchor = parent.clone().upcast::<gtk::Widget>();
        choose.connect_clicked(move |_| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("image/*");
            filter.set_name(Some(tr("Images")));
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let picker = gtk::FileDialog::new();
            picker.set_title(tr("Choose image…"));
            picker.set_filters(Some(&filters));
            picker.set_default_filter(Some(&filter));
            let window = anchor.root().and_downcast::<gtk::Window>();
            let apply = apply.clone();
            let path = path.clone();
            glib::spawn_future_local(async move {
                if let Ok(file) = picker.open_future(window.as_ref()).await {
                    if let Some(picture) = file.path() {
                        apply(set_picture(&path, &picture));
                    }
                }
            });
        });
    }

    refresh();
    let dialog = ndl::Dialog::builder()
        .title(tr("Customize folder"))
        .child(&body)
        .content_width(440)
        .build();
    {
        let dialog = dialog.clone();
        close.connect_clicked(move |_| dialog.force_close());
    }
    dialog.set_valign(gtk::Align::Center);
    dialog.present(Some(parent));
}
