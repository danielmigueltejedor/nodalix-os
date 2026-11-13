//! A folder's contents: reading, ordering and filtering them.

use std::cmp::Ordering;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::names::natural_cmp;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    /// A folder, or a link that leads to one.
    pub is_dir: bool,
    pub is_link: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// The folder's `.hidden` file names it.
    pub listed_hidden: bool,
}

impl Entry {
    pub fn is_hidden(&self) -> bool {
        self.listed_hidden || self.name.starts_with('.')
    }

    /// The extension without its dot, lower case; empty for folders and names without one.
    pub fn extension(&self) -> String {
        if self.is_dir {
            return String::new();
        }
        match self.name.rfind('.') {
            Some(i) if i > 0 => self.name[i + 1..].to_lowercase(),
            _ => String::new(),
        }
    }

    pub(crate) fn at(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        let link = path.symlink_metadata().ok()?;
        let is_link = link.file_type().is_symlink();
        // What a link points at decides how it is drawn and opened; a dangling one is a file.
        let meta = if is_link {
            path.metadata().unwrap_or(link)
        } else {
            link
        };
        Some(Self {
            name,
            is_dir: meta.is_dir(),
            is_link,
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified: meta.modified().ok(),
            listed_hidden: false,
            path,
        })
    }
}

/// Reads a folder. Entries that vanish while it is read are left out.
///
/// Names in the folder's `.hidden` file (one per line, the way GNOME's file managers read it) are
/// marked as hidden, so the aliases Nodalix keeps for old folder names do not show up twice.
pub fn read_dir(path: &Path) -> io::Result<Vec<Entry>> {
    let hidden = std::fs::read_to_string(path.join(".hidden")).unwrap_or_default();
    let hidden: Vec<&str> = hidden.lines().map(str::trim).collect();
    Ok(std::fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|e| Entry::at(e.path()))
        .map(|mut entry| {
            entry.listed_hidden = hidden.contains(&entry.name.as_str());
            entry
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Modified,
    Size,
    Kind,
}

/// Folders first, then by `key`; ties fall back to the name so the order is total.
pub fn compare(a: &Entry, b: &Entry, key: SortKey, descending: bool) -> Ordering {
    let by_key = match key {
        SortKey::Name => natural_cmp(&a.name, &b.name),
        SortKey::Modified => a.modified.cmp(&b.modified),
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::Kind => a.extension().cmp(&b.extension()),
    };
    let by_key = if descending { by_key.reverse() } else { by_key };
    b.is_dir
        .cmp(&a.is_dir)
        .then(by_key)
        .then_with(|| natural_cmp(&a.name, &b.name))
        .then_with(|| a.name.cmp(&b.name))
}

/// Whether an entry shows: hidden ones only on request, and only those whose name contains the
/// search text (case does not matter).
pub fn matches(entry: &Entry, show_hidden: bool, search: &str) -> bool {
    if entry.is_hidden() && !show_hidden {
        return false;
    }
    search.is_empty() || entry.name.to_lowercase().contains(&search.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn entry(name: &str, is_dir: bool, size: u64, secs: u64) -> Entry {
        Entry {
            name: name.into(),
            path: PathBuf::from("/x").join(name),
            is_dir,
            is_link: false,
            size,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
            listed_hidden: false,
        }
    }

    fn sorted(mut v: Vec<Entry>, key: SortKey, desc: bool) -> Vec<String> {
        v.sort_by(|a, b| compare(a, b, key, desc));
        v.into_iter().map(|e| e.name).collect()
    }

    #[test]
    fn the_hidden_file_hides_what_it_names() {
        let dir = std::env::temp_dir().join(format!("archivador-hidden-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Aplicaciones")).unwrap();
        std::os::unix::fs::symlink("Aplicaciones", dir.join("Applications")).unwrap();
        std::fs::write(dir.join(".hidden"), "Applications\n  \nAbsent\n").unwrap();
        let entries = read_dir(&dir).unwrap();
        let shown: Vec<&str> = entries
            .iter()
            .filter(|e| matches(e, false, ""))
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(shown, ["Aplicaciones"]);
        // Showing hidden files shows it again.
        assert!(entries
            .iter()
            .any(|e| matches(e, true, "") && e.name == "Applications"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn folders_come_first_whatever_the_order() {
        let v = vec![
            entry("b.txt", false, 5, 1),
            entry("zdir", true, 0, 1),
            entry("a.txt", false, 9, 2),
        ];
        assert_eq!(
            sorted(v.clone(), SortKey::Name, false),
            ["zdir", "a.txt", "b.txt"]
        );
        assert_eq!(sorted(v, SortKey::Name, true), ["zdir", "b.txt", "a.txt"]);
    }

    #[test]
    fn size_modified_and_kind_orders() {
        let v = vec![
            entry("a.png", false, 30, 3),
            entry("b.txt", false, 10, 1),
            entry("c.md", false, 20, 2),
        ];
        assert_eq!(
            sorted(v.clone(), SortKey::Size, false),
            ["b.txt", "c.md", "a.png"]
        );
        assert_eq!(
            sorted(v.clone(), SortKey::Modified, true),
            ["a.png", "c.md", "b.txt"]
        );
        assert_eq!(sorted(v, SortKey::Kind, false), ["c.md", "a.png", "b.txt"]);
    }

    #[test]
    fn equal_keys_fall_back_to_the_name() {
        let v = vec![
            entry("b", false, 1, 1),
            entry("a", false, 1, 1),
            entry("B", false, 1, 1),
        ];
        assert_eq!(sorted(v, SortKey::Size, false), ["a", "B", "b"]);
    }

    #[test]
    fn hidden_entries_and_the_search_text_filter_the_listing() {
        let hidden = entry(".config", true, 0, 1);
        let shown = entry("Report.PDF", false, 1, 1);
        assert!(!matches(&hidden, false, ""));
        assert!(matches(&hidden, true, ""));
        assert!(matches(&shown, false, "report"));
        assert!(!matches(&shown, false, "xyz"));
    }

    #[test]
    fn extensions_are_lower_case_and_folders_have_none() {
        assert_eq!(entry("Photo.JPG", false, 1, 1).extension(), "jpg");
        assert_eq!(entry(".bashrc", false, 1, 1).extension(), "");
        assert_eq!(entry("v1.2", true, 0, 1).extension(), "");
    }

    #[test]
    fn a_folder_is_read_with_links_and_dangling_links() {
        let dir = std::env::temp_dir().join(format!("archivador-entry-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("file.txt"), "hello").unwrap();
        std::os::unix::fs::symlink(dir.join("sub"), dir.join("to-sub")).unwrap();
        std::os::unix::fs::symlink("/nonexistent", dir.join("dangling")).unwrap();
        let mut entries = read_dir(&dir).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let by = |n: &str| entries.iter().find(|e| e.name == n).unwrap();
        assert_eq!(entries.len(), 4);
        assert!(by("sub").is_dir && !by("sub").is_link);
        assert!(by("to-sub").is_dir && by("to-sub").is_link);
        assert!(!by("dangling").is_dir && by("dangling").is_link);
        assert_eq!(by("file.txt").size, 5);
        assert!(read_dir(&dir.join("missing")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
