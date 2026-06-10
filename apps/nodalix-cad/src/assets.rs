//! Runtime asset paths (icons). Works when installed to `~/.local/bin/lixcad`.
//!
//! Icon naming convention:
//! - `line`, `move` → `assets/icons/{name}.svg`
//! - `snap-endpoint` → `assets/icons/snaps/snap-endpoint.svg` (or flat `icons/snap-endpoint.svg`)
//! - `precision/precision-ortho` → `assets/icons/precision/precision-ortho.svg`
//! - `modify/modify-rotate` → `assets/icons/modify/modify-rotate.svg`

use gtk::prelude::*;
use std::path::{Path, PathBuf};

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

/// Resolve icon path: supports `subdir/name` and automatic `snaps/` for `snap-*` names.
pub fn icon_path(name: &str) -> PathBuf {
    let dir = icons_dir();
    let mut candidates = icon_path_candidates(&dir, name);
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    if bundled != dir && bundled.is_dir() {
        candidates.extend(icon_path_candidates(&bundled, name));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| dir.join(format!("{name}.svg")))
}

fn icon_path_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut paths = vec![dir.join(format!("{name}.svg"))];
    if name.contains('/') {
        return paths;
    }
    if name.starts_with("snap-") {
        paths.push(dir.join("snaps").join(format!("{name}.svg")));
    }
    if name.starts_with("modify-") {
        paths.push(dir.join("modify").join(format!("{name}.svg")));
    }
    if name.starts_with("draw-") {
        paths.push(dir.join("draw").join(format!("{name}.svg")));
    }
    if name.starts_with("hatch-") {
        paths.push(dir.join("hatch").join(format!("{name}.svg")));
    }
    if name.starts_with("block-") {
        paths.push(dir.join("block").join(format!("{name}.svg")));
    }
    if name == "block-insert" {
        paths.push(dir.join("block").join("block-insert.svg"));
    }
    if name.starts_with("precision-") {
        paths.push(dir.join("precision").join(format!("{name}.svg")));
    }
    paths
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

    #[test]
    fn snap_endpoint_icon_resolves() {
        let path = icon_path("snap-endpoint");
        assert!(
            path.is_file(),
            "snap-endpoint icon missing: {}",
            path.display()
        );
    }

    #[test]
    fn snap_toggle_icon_resolves() {
        let path = icon_path("snap-toggle");
        assert!(
            path.is_file(),
            "snap-toggle icon missing: {}",
            path.display()
        );
    }

    #[test]
    fn all_osnap_icons_resolve() {
        for name in [
            "snap-toggle",
            "snap-endpoint",
            "snap-midpoint",
            "snap-center",
            "snap-intersection",
            "snap-quadrant",
            "snap-nearest",
            "snap-node",
            "snap-perpendicular",
            "snap-tangent",
        ] {
            let path = icon_path(name);
            assert!(path.is_file(), "missing icon: {}", path.display());
        }
    }

    #[test]
    fn hatch_icons_resolve() {
        for name in ["draw-hatch", "hatch-solid", "hatch-ansi31"] {
            let path = icon_path(name);
            assert!(path.is_file(), "missing icon: {}", path.display());
        }
    }

    #[test]
    fn export_pdf_icon_resolves() {
        let path = icon_path("export-pdf");
        assert!(path.is_file(), "missing icon: {}", path.display());
    }

    #[test]
    fn block_icons_resolve() {
        for name in [
            "block-insert",
            "block-create",
            "block-explode",
            "block-manager",
        ] {
            let path = icon_path(name);
            assert!(path.is_file(), "missing icon: {}", path.display());
        }
    }

    #[test]
    fn modify_icons_resolve() {
        for name in [
            "modify-rotate",
            "modify-scale",
            "modify-mirror",
            "modify-move",
            "modify-copy",
            "modify-offset",
            "modify-trim",
            "modify-extend",
            "modify-fillet",
            "modify-chamfer",
        ] {
            let path = icon_path(name);
            assert!(path.is_file(), "missing icon: {}", path.display());
        }
    }
}
