//! Moving files between places: through the system clipboard and by drag and drop, in the formats
//! other file managers use, so a copy in Archivador pastes in any of them and the other way round.

use std::path::PathBuf;

use archivador_core::clipboard::{Clipboard, Mode};
use ndl::gtk::prelude::*;
use ndl::gtk::{self, gdk, gio, glib};
use nodalia_gtk as ndl;

/// GNOME's own flavour: the first line says `copy` or `cut`, then one URI per line.
const GNOME_COPIED: &str = "x-special/gnome-copied-files";

fn files(paths: &[PathBuf]) -> Vec<gio::File> {
    paths.iter().map(gio::File::for_path).collect()
}

/// What goes on the clipboard for a copy or a cut: the files themselves, GNOME's list that also
/// says which of the two it is, and plain URIs for the apps that read only those.
pub fn clipboard_content(mode: Mode, paths: &[PathBuf]) -> gdk::ContentProvider {
    let files = files(paths);
    let mut gnome = String::from(if mode == Mode::Cut { "cut" } else { "copy" });
    let mut uris = String::new();
    for file in &files {
        gnome.push('\n');
        gnome.push_str(&file.uri());
        uris.push_str(&file.uri());
        uris.push_str("\r\n");
    }
    gdk::ContentProvider::new_union(&[
        gdk::ContentProvider::for_value(&gdk::FileList::from_array(&files).to_value()),
        gdk::ContentProvider::for_bytes(GNOME_COPIED, &glib::Bytes::from_owned(gnome.into_bytes())),
        gdk::ContentProvider::for_bytes(
            "text/uri-list",
            &glib::Bytes::from_owned(uris.into_bytes()),
        ),
    ])
}

/// What a drag carries.
pub fn drag_content(paths: &[PathBuf]) -> gdk::ContentProvider {
    gdk::ContentProvider::for_value(&gdk::FileList::from_array(&files(paths)).to_value())
}

/// Whether the clipboard holds files.
pub fn has_files(clipboard: &gdk::Clipboard) -> bool {
    clipboard
        .formats()
        .contains_type(gdk::FileList::static_type())
}

/// The paths in a dropped file list.
pub fn paths_of(value: &glib::Value) -> Vec<PathBuf> {
    value
        .get::<gdk::FileList>()
        .map(|list| list.files().iter().filter_map(gio::File::path).collect())
        .unwrap_or_default()
}

/// The files on the clipboard and whether they were copied or cut, if it holds files at all.
pub async fn read(clipboard: &gdk::Clipboard) -> Option<Clipboard> {
    if !has_files(clipboard) {
        return None;
    }
    let value = clipboard
        .read_value_future(gdk::FileList::static_type(), glib::Priority::DEFAULT)
        .await
        .ok()?;
    let paths = paths_of(&value);
    if paths.is_empty() {
        return None;
    }
    let mut mode = Mode::Copy;
    if clipboard.formats().contain_mime_type(GNOME_COPIED) {
        if let Ok((stream, _)) = clipboard
            .read_future(&[GNOME_COPIED], glib::Priority::DEFAULT)
            .await
        {
            if let Ok(bytes) = stream
                .read_bytes_future(65536, glib::Priority::DEFAULT)
                .await
            {
                if bytes.starts_with(b"cut") {
                    mode = Mode::Cut;
                }
            }
        }
    }
    Some(Clipboard::new(mode, paths))
}

/// Whether a drop means copy rather than move: a drag from another app is copied, one of ours
/// moves unless Ctrl is held. (The negotiated action is no help: GTK settles on a copy whenever
/// both are allowed.)
pub fn drop_copies(target: &gtk::DropTarget) -> bool {
    let Some(drop) = target.current_drop() else {
        return false;
    };
    if drop.drag().is_none() {
        return true;
    }
    drop.display()
        .default_seat()
        .and_then(|seat| seat.keyboard())
        .is_some_and(|keyboard| {
            keyboard
                .modifier_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
        })
}
