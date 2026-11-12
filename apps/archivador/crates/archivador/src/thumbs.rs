//! Thumbnails of images, made off the main thread and remembered while the app runs.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use archivador_core::entry::Entry;
use ndl::gtk::gdk_pixbuf::Pixbuf;
use ndl::gtk::{gdk, gio, glib};
use nodalia_gtk as ndl;

/// Past this size a file is not worth decoding just to draw a thumbnail.
const LARGEST: u64 = 40_000_000;
/// How many thumbnails are kept; the oldest go first.
const KEPT: usize = 400;

const EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "tif", "tiff",
];

type Key = (PathBuf, u64, i32);

thread_local! {
    static CACHE: RefCell<(Vec<Key>, HashMap<Key, gdk::Texture>)> = RefCell::default();
}

/// Whether a thumbnail can be made for this file.
pub fn wanted(entry: &Entry) -> bool {
    !entry.is_dir && entry.size <= LARGEST && EXTENSIONS.contains(&entry.extension().as_str())
}

fn key(entry: &Entry, size: i32) -> Key {
    let modified = entry
        .modified
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    (entry.path.clone(), modified, size)
}

/// The thumbnail if it is already made.
pub fn cached(entry: &Entry, size: i32) -> Option<gdk::Texture> {
    CACHE.with(|c| c.borrow().1.get(&key(entry, size)).cloned())
}

/// Makes the thumbnail, `size` pixels at most on a side, and hands it to `ready`. Nothing is
/// called back when the file cannot be decoded.
pub fn load(entry: &Entry, size: i32, ready: impl FnOnce(gdk::Texture) + 'static) {
    if let Some(texture) = cached(entry, size) {
        ready(texture);
        return;
    }
    let key = key(entry, size);
    let path = entry.path.clone();
    glib::spawn_future_local(async move {
        // The decoder runs on a worker; only bytes cross back, since pixbufs stay where they
        // were made.
        let png = gio::spawn_blocking(move || {
            let pixbuf = Pixbuf::from_file_at_scale(&path, size, size, true).ok()?;
            let pixbuf = pixbuf.apply_embedded_orientation().unwrap_or(pixbuf);
            pixbuf.save_to_bufferv("png", &[]).ok()
        })
        .await
        .ok()
        .flatten();
        let Some(png) = png else { return };
        let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_owned(png)) else {
            return;
        };
        CACHE.with(|c| {
            let (order, map) = &mut *c.borrow_mut();
            if map.insert(key.clone(), texture.clone()).is_none() {
                order.push(key);
            }
            if order.len() > KEPT {
                let oldest = order.remove(0);
                map.remove(&oldest);
            }
        });
        ready(texture);
    });
}
