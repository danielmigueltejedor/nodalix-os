//! The folders the user pinned to the sidebar, in the file GTK has always kept them in
//! (`~/.config/gtk-3.0/bookmarks`), so they are the same ones in every file manager.
//!
//! One per line: `file:///percent/encoded/path` and, after a space, an optional label.

use std::path::{Path, PathBuf};

use crate::trash::{decode_path, encode_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub path: PathBuf,
    /// What the sidebar calls it; the folder's name when the file gives none.
    pub label: String,
}

/// The bookmarks in `text`. Lines that are not `file://` locations (a network place another
/// program pinned, say) are not ours to show and are skipped.
pub fn parse(text: &str) -> Vec<Bookmark> {
    let mut found: Vec<Bookmark> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let (uri, label) = line.split_once(' ').unwrap_or((line, ""));
        let Some(encoded) = uri.strip_prefix("file://") else {
            continue;
        };
        let path = decode_path(encoded);
        if !path.is_absolute() || found.iter().any(|b| b.path == path) {
            continue;
        }
        let label = match label.trim() {
            "" => path
                .file_name()
                .map_or_else(|| "/".to_owned(), |n| n.to_string_lossy().into_owned()),
            label => label.to_owned(),
        };
        found.push(Bookmark { path, label });
    }
    found
}

/// The file's text for `bookmarks`; a label is written only when it is not just the folder's name.
pub fn serialize(bookmarks: &[Bookmark]) -> String {
    let mut out = String::new();
    for bookmark in bookmarks {
        out.push_str("file://");
        out.push_str(&encode_path(&bookmark.path));
        let plain = bookmark
            .path
            .file_name()
            .is_some_and(|n| n.to_string_lossy() == bookmark.label);
        if !plain && !bookmark.label.is_empty() {
            out.push(' ');
            out.push_str(&bookmark.label.replace('\n', " "));
        }
        out.push('\n');
    }
    out
}

/// `bookmarks` with `path` added at the end; unchanged if it is there already.
pub fn add(mut bookmarks: Vec<Bookmark>, path: &Path) -> Vec<Bookmark> {
    if !bookmarks.iter().any(|b| b.path == path) {
        let label = path
            .file_name()
            .map_or_else(|| "/".to_owned(), |n| n.to_string_lossy().into_owned());
        bookmarks.push(Bookmark {
            path: path.to_path_buf(),
            label,
        });
    }
    bookmarks
}

/// `bookmarks` without `path`.
pub fn remove(mut bookmarks: Vec<Bookmark>, path: &Path) -> Vec<Bookmark> {
    bookmarks.retain(|b| b.path != path);
    bookmarks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paths_and_labels() {
        let found = parse("file:///home/a/My%20Docs Work\nfile:///srv/data\n");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, Path::new("/home/a/My Docs"));
        assert_eq!(found[0].label, "Work");
        assert_eq!(found[1].label, "data");
    }

    #[test]
    fn skips_other_locations_and_repeats() {
        let found = parse("smb://nas/share\nfile:///x\nfile:///x Again\n\nnot a uri\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, Path::new("/x"));
    }

    #[test]
    fn serializing_and_parsing_agree() {
        let list = vec![
            Bookmark {
                path: PathBuf::from("/home/a/My Docs"),
                label: "Work".into(),
            },
            Bookmark {
                path: PathBuf::from("/srv/data"),
                label: "data".into(),
            },
        ];
        let text = serialize(&list);
        assert_eq!(text, "file:///home/a/My%20Docs Work\nfile:///srv/data\n");
        assert_eq!(parse(&text), list);
    }

    #[test]
    fn adding_and_removing() {
        let list = add(Vec::new(), Path::new("/a/b"));
        assert_eq!(list[0].label, "b");
        assert_eq!(add(list.clone(), Path::new("/a/b")), list);
        assert!(remove(list, Path::new("/a/b")).is_empty());
    }
}
