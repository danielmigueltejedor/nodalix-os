//! The icons of files and folders: full-colour, from the system's icon theme.
//!
//! The Nodalia design language's own icon theme draws only symbolic icons, and GTK prefers a
//! theme's `folder-symbolic` over the inherited theme's coloured `folder`, so a file shown through
//! the window's theme turns into a flat glyph, like the sidebar's. Files and folders are looked up
//! here instead, in a theme of their own: the one the system has chosen.

use archivador_core::entry::Entry;
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

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

/// The folders the user has, which GNOME and Colloid draw with a picture of what they hold.
fn folder_icon_name(path: &std::path::Path) -> &'static str {
    use glib::UserDirectory as Dir;
    const SPECIAL: [(Dir, &str); 8] = [
        (Dir::Documents, "folder-documents"),
        (Dir::Downloads, "folder-download"),
        (Dir::Music, "folder-music"),
        (Dir::Pictures, "folder-pictures"),
        (Dir::Videos, "folder-videos"),
        (Dir::Templates, "folder-templates"),
        (Dir::PublicShare, "folder-publicshare"),
        (Dir::Desktop, "user-desktop"),
    ];
    if glib::home_dir() == path {
        return "user-home";
    }
    SPECIAL
        .iter()
        .find(|(dir, _)| glib::user_special_dir(*dir).is_some_and(|p| p == path))
        .map_or("folder", |(_, name)| name)
}

/// The names to try for this file, most specific first.
pub fn icon_for(entry: &Entry) -> gio::Icon {
    if entry.is_dir {
        return gio::ThemedIcon::from_names(&[folder_icon_name(&entry.path), "folder"]).upcast();
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

/// The columns view's glyph, which no icon theme has: three columns, drawn like the NDL icons.
const COLUMNS_GLYPH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><rect class="transparent-fill foreground-stroke" fill="none" stroke="rgb(0,0,0)" stroke-width="2.25" stroke-linejoin="round" x="3.5" y="4.5" width="17" height="15" rx="2"/><path class="transparent-fill foreground-stroke" fill="none" stroke="rgb(0,0,0)" stroke-width="2.25" stroke-linecap="round" d="M9.2 4.5v15M14.8 4.5v15"/></svg>"#;

/// An image of that glyph, in the colour of the text around it.
pub fn columns_glyph() -> gtk::Image {
    let path = glib::user_cache_dir()
        .join("nodalix")
        .join("archivador")
        .join("view-columns-symbolic.svg");
    let written = path.exists()
        || path
            .parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
            && std::fs::write(&path, COLUMNS_GLYPH).is_ok();
    if written {
        let icon = gtk::IconPaintable::for_file(&gio::File::for_path(&path), 16, 1);
        let image = gtk::Image::from_paintable(Some(&icon));
        image.set_pixel_size(16);
        image
    } else {
        gtk::Image::from_icon_name("view-dual-symbolic")
    }
}
