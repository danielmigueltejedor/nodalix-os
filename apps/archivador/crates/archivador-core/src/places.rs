//! The places in the sidebar, and the path bar's segments.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// English title, which the window translates.
    pub title: &'static str,
    pub icon: &'static str,
    pub path: PathBuf,
}

/// The standard user folders, in the order the sidebar lists them. `user_dirs` is the text of
/// `~/.config/user-dirs.dirs`; a folder that is missing from it takes its usual name in the home
/// folder, and one that does not exist is left out (`exists` says which do).
pub fn standard(home: &Path, user_dirs: &str, exists: impl Fn(&Path) -> bool) -> Vec<Place> {
    const KINDS: [(&str, &str, &str, &str); 6] = [
        ("DESKTOP", "Desktop", "user-desktop-symbolic", "Desktop"),
        (
            "DOCUMENTS",
            "Documents",
            "folder-documents-symbolic",
            "Documents",
        ),
        (
            "DOWNLOAD",
            "Downloads",
            "folder-download-symbolic",
            "Downloads",
        ),
        ("MUSIC", "Music", "folder-music-symbolic", "Music"),
        (
            "PICTURES",
            "Pictures",
            "folder-pictures-symbolic",
            "Pictures",
        ),
        ("VIDEOS", "Videos", "folder-videos-symbolic", "Videos"),
    ];
    let mut places = vec![Place {
        title: "Home",
        icon: "user-home-symbolic",
        path: home.to_path_buf(),
    }];
    for (key, title, icon, fallback) in KINDS {
        let path = user_dir(user_dirs, key, home).unwrap_or_else(|| home.join(fallback));
        // A folder the user moved to the home folder itself is not a place of its own.
        if path != home && exists(&path) {
            places.push(Place { title, icon, path });
        }
    }
    places
}

/// The Applications folder, which is one of Nodalix's own places. `layout` is the text of
/// `~/.local/state/nodalix/layout/paths.json`, where Nodalix's layout service records which of
/// `Aplicaciones` and `Applications` is the real one (the other is a link to it); without it the
/// first of the two that exists is taken.
pub fn applications(home: &Path, layout: &str, exists: impl Fn(&Path) -> bool) -> Option<Place> {
    let recorded = serde_json::from_str::<serde_json::Value>(layout)
        .ok()
        .and_then(|v| v.get("applications")?.as_str().map(PathBuf::from))
        .filter(|p| p.parent() == Some(home) && exists(p));
    let path = recorded.or_else(|| {
        ["Aplicaciones", "Applications"]
            .iter()
            .map(|name| home.join(name))
            .find(|p| exists(p))
    })?;
    Some(Place {
        title: "Applications",
        icon: "view-app-grid-symbolic",
        path,
    })
}

/// `XDG_<key>_DIR="$HOME/Music"` in `user-dirs.dirs`.
fn user_dir(text: &str, key: &str, home: &Path) -> Option<PathBuf> {
    let prefix = format!("XDG_{key}_DIR=");
    let value = text
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))?
        .trim()
        .trim_matches('"');
    if let Some(rest) = value.strip_prefix("$HOME") {
        let rest = rest.trim_start_matches('/');
        return Some(if rest.is_empty() {
            home.to_path_buf()
        } else {
            home.join(rest)
        });
    }
    value.starts_with('/').then(|| PathBuf::from(value))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    /// What the button says; `None` is the home folder, which the window names.
    pub label: Option<String>,
    pub path: PathBuf,
}

/// The path bar: one segment per folder from the root, or from the home folder when the path is
/// inside it. The root is a segment labelled `/`.
pub fn breadcrumbs(path: &Path, home: &Path) -> Vec<Crumb> {
    let mut crumbs = Vec::new();
    let (mut at, rest) = match path.strip_prefix(home) {
        Ok(rest) => {
            crumbs.push(Crumb {
                label: None,
                path: home.to_path_buf(),
            });
            (home.to_path_buf(), rest.to_path_buf())
        }
        Err(_) => {
            crumbs.push(Crumb {
                label: Some("/".into()),
                path: PathBuf::from("/"),
            });
            (
                PathBuf::from("/"),
                path.strip_prefix("/").unwrap_or(path).to_path_buf(),
            )
        }
    };
    for part in rest.components() {
        at.push(part);
        crumbs.push(Crumb {
            label: Some(part.as_os_str().to_string_lossy().into_owned()),
            path: at.clone(),
        });
    }
    crumbs
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRS: &str = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Escritorio\"\nXDG_DOWNLOAD_DIR=\"$HOME/Descargas\"\nXDG_MUSIC_DIR=\"$HOME\"\nXDG_PICTURES_DIR=\"/data/pics\"\n";

    #[test]
    fn user_dirs_set_the_names_and_missing_ones_use_the_usual_name() {
        let home = Path::new("/home/a");
        let places = standard(home, DIRS, |_| true);
        let paths: Vec<_> = places
            .iter()
            .map(|p| (p.title, p.path.to_str().unwrap()))
            .collect();
        assert_eq!(
            paths,
            [
                ("Home", "/home/a"),
                ("Desktop", "/home/a/Escritorio"),
                ("Documents", "/home/a/Documents"),
                ("Downloads", "/home/a/Descargas"),
                // Music is the home folder itself: not a place of its own.
                ("Pictures", "/data/pics"),
                ("Videos", "/home/a/Videos"),
            ]
        );
    }

    #[test]
    fn the_applications_folder_is_the_recorded_one_or_the_first_that_exists() {
        let home = Path::new("/home/a");
        let layout = r#"{"applications": "/home/a/Applications"}"#;
        let place = applications(home, layout, |_| true).unwrap();
        assert_eq!(place.path, Path::new("/home/a/Applications"));
        let place = applications(home, "", |_| true).unwrap();
        assert_eq!(place.path, Path::new("/home/a/Aplicaciones"));
        let place = applications(home, "", |p| p.ends_with("Applications")).unwrap();
        assert_eq!(place.path, Path::new("/home/a/Applications"));
        // A record that points outside the home folder, or at nothing, is not trusted.
        let elsewhere = r#"{"applications": "/etc"}"#;
        let place = applications(home, elsewhere, |p| p.ends_with("Aplicaciones")).unwrap();
        assert_eq!(place.path, Path::new("/home/a/Aplicaciones"));
        assert!(applications(home, "", |_| false).is_none());
    }

    #[test]
    fn folders_that_do_not_exist_are_left_out() {
        let places = standard(Path::new("/home/a"), "", |p| p.ends_with("Documents"));
        let titles: Vec<_> = places.iter().map(|p| p.title).collect();
        assert_eq!(titles, ["Home", "Documents"]);
    }

    #[test]
    fn the_path_bar_starts_at_home_inside_it_and_at_the_root_elsewhere() {
        let home = Path::new("/home/a");
        let inside = breadcrumbs(Path::new("/home/a/docs/work"), home);
        assert_eq!(inside.len(), 3);
        assert_eq!(
            inside[0],
            Crumb {
                label: None,
                path: home.into()
            }
        );
        assert_eq!(inside[2].label.as_deref(), Some("work"));
        assert_eq!(inside[2].path, Path::new("/home/a/docs/work"));
        let outside = breadcrumbs(Path::new("/usr/share"), home);
        let labels: Vec<_> = outside.iter().map(|c| c.label.clone().unwrap()).collect();
        assert_eq!(labels, ["/", "usr", "share"]);
        assert_eq!(breadcrumbs(home, home).len(), 1);
        assert_eq!(breadcrumbs(Path::new("/"), home).len(), 1);
    }
}
