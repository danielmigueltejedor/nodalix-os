//! The properties of what is selected: what it is, how big, where, when, who may do what.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use archivador_core::entry::Entry;
use archivador_core::{format, ops};
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::i18n::{fmt, items, tr};

fn date(secs: i64) -> String {
    glib::DateTime::from_unix_local(secs)
        .ok()
        .and_then(|d| d.format("%e %b %Y, %H:%M").ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn value(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    label.set_selectable(true);
    label.set_max_width_chars(36);
    label
}

fn kind(entry: &Entry) -> String {
    if entry.is_dir {
        return tr("Folder").to_owned();
    }
    let (content_type, _) = gio::content_type_guess(Some(&entry.name), None::<&[u8]>);
    gio::content_type_get_description(&content_type).to_string()
}

fn size_text(bytes: u64, files: u64) -> String {
    let size = format::size(bytes);
    if files > 1 {
        format!("{size} · {}", fmt("{} files", &files.to_string()))
    } else {
        size
    }
}

/// Opens the dialog for `entries`. A folder's size is added up in the background.
pub fn present(parent: &impl IsA<gtk::Widget>, entries: Vec<Entry>) {
    let Some(first) = entries.first().cloned() else {
        return;
    };
    let single = entries.len() == 1;

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.add_css_class("archivador-properties");

    let icon = gtk::Image::new();
    icon.set_pixel_size(64);
    if single {
        if first.is_dir {
            icon.set_icon_name(Some("folder"));
        } else {
            let (content_type, _) = gio::content_type_guess(Some(&first.name), None::<&[u8]>);
            icon.set_from_gicon(&gio::content_type_get_icon(&content_type));
        }
    } else {
        icon.set_icon_name(Some("edit-copy-symbolic"));
    }
    let title = gtk::Label::new(Some(&if single {
        first.name.clone()
    } else {
        items(entries.len())
    }));
    title.add_css_class("title-3");
    title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    title.set_selectable(true);
    title.set_max_width_chars(30);
    title.set_justify(gtk::Justification::Center);
    body.append(&icon);
    body.append(&title);

    let grid = gtk::Grid::new();
    grid.set_row_spacing(8);
    grid.set_column_spacing(16);
    let mut row = 0;
    let mut add = |name: &str, widget: &gtk::Label| {
        let label = gtk::Label::new(Some(name));
        label.add_css_class("dim-label");
        label.set_xalign(1.0);
        label.set_yalign(0.0);
        grid.attach(&label, 0, row, 1, 1);
        grid.attach(widget, 1, row, 1, 1);
        row += 1;
    };

    let parents: Vec<Option<&Path>> = entries.iter().map(|e| e.path.parent()).collect();
    if single {
        add(tr("Type"), &value(&kind(&first)));
    }
    let size = value(tr("Calculating…"));
    add(tr("Size"), &size);
    if parents.iter().all(|p| *p == parents[0]) {
        if let Some(dir) = parents[0] {
            add(tr("Location"), &value(&dir.to_string_lossy()));
        }
    }
    if single {
        if let Ok(meta) = first.path.symlink_metadata() {
            add(tr("Modified"), &value(&date(meta.mtime())));
            add(tr("Accessed"), &value(&date(meta.atime())));
            add(tr("Permissions"), &value(&format::permissions(meta.mode())));
        }
    }
    body.append(&grid);

    let close = gtk::Button::with_label(tr("Close"));
    close.set_halign(gtk::Align::Center);
    close.set_margin_top(4);
    body.append(&close);

    let dialog = ndl::Dialog::builder()
        .title(tr("Properties"))
        .child(&body)
        .content_width(420)
        .build();
    {
        let dialog = dialog.clone();
        close.connect_clicked(move |_| dialog.force_close());
    }
    dialog.set_valign(gtk::Align::Center);
    dialog.present(Some(parent));

    // Adding up a big folder takes a while: do it off the main thread.
    let paths: Vec<PathBuf> = entries.into_iter().map(|e| e.path).collect();
    glib::spawn_future_local(async move {
        let totals = gio::spawn_blocking(move || {
            paths.iter().fold((0, 0), |(bytes, files), path| {
                let (b, f) = ops::tree_size(path);
                (bytes + b, files + f)
            })
        })
        .await;
        match totals {
            Ok((bytes, files)) => size.set_label(&size_text(bytes, files)),
            Err(_) => size.set_label("?"),
        }
    });
}
