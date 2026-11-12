//! The icons of files and folders: full-colour, from the system's icon theme.
//!
//! The Nodalia design language's own icon theme draws only symbolic icons, and GTK prefers a
//! theme's `folder-symbolic` over the inherited theme's coloured `folder`, so a file shown through
//! the window's theme turns into a flat glyph, like the sidebar's. Files and folders are looked up
//! here instead, in a theme of their own: the one the system has chosen.

use archivador_core::entry::Entry;
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::folders::{self, Kind};

const SCHEMA: &str = "org.gnome.desktop.interface";
const KEY: &str = "icon-theme";

thread_local! {
    static THEME: gtk::IconTheme = system_theme();
}

/// An icon theme that follows the system's choice, not the window's.
fn system_theme() -> gtk::IconTheme {
    let theme = gtk::IconTheme::new();
    let settings = gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SCHEMA, true))
        .map(|schema| gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None));
    let name = |settings: &Option<gio::Settings>| {
        settings
            .as_ref()
            .map(|s| s.string(KEY).to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Adwaita".to_owned())
    };
    theme.set_theme_name(Some(&name(&settings)));
    if let Some(settings) = settings {
        let theme = theme.clone();
        settings.connect_changed(Some(KEY), move |settings, key| {
            theme.set_theme_name(Some(settings.string(key).as_str()));
        });
        // Lives as long as the process.
        std::mem::forget(settings);
    }
    theme
}

/// Which kind of folder this is: one of the user's own, which carry a picture of what they
/// hold, the Applications folder, or any other.
fn folder_kind(path: &std::path::Path) -> Kind {
    use glib::UserDirectory as Dir;
    const SPECIAL: [(Dir, Kind); 8] = [
        (Dir::Documents, Kind::Documents),
        (Dir::Downloads, Kind::Downloads),
        (Dir::Music, Kind::Music),
        (Dir::Pictures, Kind::Pictures),
        (Dir::Videos, Kind::Videos),
        (Dir::Templates, Kind::Templates),
        (Dir::PublicShare, Kind::Public),
        (Dir::Desktop, Kind::Desktop),
    ];
    let home = glib::home_dir();
    if path.parent() == Some(home.as_path())
        && path
            .file_name()
            .is_some_and(|n| n == "Aplicaciones" || n == "Applications")
    {
        return Kind::Applications;
    }
    if home == path {
        return Kind::Plain;
    }
    SPECIAL
        .iter()
        .find(|(dir, _)| glib::user_special_dir(*dir).is_some_and(|p| p == path))
        .map_or(Kind::Plain, |(_, kind)| *kind)
}

/// The picture the file carries as its own: the icon GNOME's file managers keep in the file's
/// `metadata::custom-icon`, as Nodalix's Applications folder does for each of its apps.
fn custom_icon(entry: &Entry) -> Option<gio::Icon> {
    const KEY: &str = "metadata::custom-icon";
    let info = gio::File::for_path(&entry.path)
        .query_info(KEY, gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE)
        .ok()?;
    let value = info.attribute_string(KEY)?;
    let file = if value.starts_with('/') {
        gio::File::for_path(value.as_str())
    } else if value.starts_with("file:") {
        gio::File::for_uri(&value)
    } else {
        return None;
    };
    file.query_exists(gio::Cancellable::NONE)
        .then(|| gio::FileIcon::new(&file).upcast())
}

/// The names to try for this file, most specific first.
pub fn icon_for(entry: &Entry) -> gio::Icon {
    if entry.is_dir {
        let kind = folder_kind(&entry.path);
        // The Applications folder carries a picture made from the theme's blue folder; a folder
        // the user gave a picture of their own keeps it. The rest are Nodalia's.
        let own = (kind != Kind::Applications)
            .then(|| custom_icon(entry))
            .flatten();
        return own.unwrap_or_else(|| folders::icon(kind));
    }
    if let Some(icon) = custom_icon(entry) {
        return icon;
    }
    let (content_type, _) = gio::content_type_guess(Some(&entry.name), None::<&[u8]>);
    gio::content_type_get_icon(&content_type)
}

/// Shows the file's own icon, in colour, at the size the image asks for.
pub fn show(image: &gtk::Image, entry: &Entry) {
    let size = match image.pixel_size() {
        n if n > 0 => n,
        _ => 48,
    };
    let scale = image.scale_factor().max(1);
    let icon = THEME.with(|theme| {
        theme.lookup_by_gicon(
            &icon_for(entry),
            size,
            scale,
            image.direction(),
            gtk::IconLookupFlags::empty(),
        )
    });
    image.set_paintable(Some(&icon));
}
