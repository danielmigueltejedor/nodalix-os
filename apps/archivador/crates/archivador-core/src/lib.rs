//! What Archivador knows, without GTK, so that it can be tested without a display.
//!
//! The window (`archivador`) only draws this and forwards what the user does.

pub mod bookmarks;
pub mod clipboard;
pub mod cloud;
pub mod entry;
pub mod format;
pub mod history;
pub mod localsend;
pub mod names;
pub mod ops;
pub mod places;
pub mod recent;
pub mod servers;
pub mod settings;
pub mod trash;
