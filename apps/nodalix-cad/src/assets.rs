//! Runtime asset paths (icons). Works when installed to `~/.local/bin/lixcad`.

use gtk::prelude::*;
use std::path::PathBuf;

/// Directory containing toolbar/palette SVG icons.
pub fn icons_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LIXCAD_ASSETS_DIR") {
        let icons = PathBuf::from(&dir).join("icons");
        if icons.is_dir() {
            return icons;
        }
        let direct = PathBuf::from(&dir);
        if direct.is_dir() {
            return direct;
        }
    }

    if let Some(home) = home_dir() {
        let installed = home.join(".local/share/nodalix-cad/assets/icons");
        if installed.is_dir() {
            return installed;
        }
    }

    let cwd = PathBuf::from("assets/icons");
    if cwd.is_dir() {
        return cwd;
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons")
}

pub fn icon_path(name: &str) -> PathBuf {
    icons_dir().join(format!("{name}.svg"))
}

pub fn load_icon_image(name: &str, size: i32) -> gtk::Image {
    let path = icon_path(name);
    let image = if path.is_file() {
        gtk::Image::from_file(&path)
    } else {
        eprintln!("Missing icon: {}", path.display());
        gtk::Image::from_icon_name("image-missing")
    };
    if !path.is_file() {
        image.add_css_class("missing-icon-fallback");
    }
    image.set_pixel_size(size);
    image.add_css_class("app-icon");
    image
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_icons_dir_exists() {
        let dir = icons_dir();
        assert!(dir.is_dir(), "icons dir missing: {}", dir.display());
        assert!(icon_path("line").is_file());
    }
}
