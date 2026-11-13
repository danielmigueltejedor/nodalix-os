//! The Recent place: the files the user opened lately, from wherever they are.

use std::path::PathBuf;
use std::time::SystemTime;

use crate::entry::Entry;

/// What the desktop remembers about one recent file.
pub struct Item {
    pub path: PathBuf,
    pub visited: Option<SystemTime>,
}

/// The files that are still there, as entries. Each one's time is when it was last opened, not
/// when it was last written, so sorting by it puts the newest visit first.
pub fn entries(items: impl IntoIterator<Item = Item>) -> Vec<Entry> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for item in items {
        if seen.contains(&item.path) {
            continue;
        }
        seen.push(item.path.clone());
        if let Some(mut entry) = Entry::at(item.path) {
            if item.visited.is_some() {
                entry.modified = item.visited;
            }
            out.push(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn missing_files_and_repeats_are_left_out() {
        let dir = std::env::temp_dir().join(format!("archivador-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let here = dir.join("a.txt");
        std::fs::write(&here, "x").unwrap();
        let visited = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let found = entries([
            Item {
                path: here.clone(),
                visited: Some(visited),
            },
            Item {
                path: here.clone(),
                visited: None,
            },
            Item {
                path: dir.join("gone.txt"),
                visited: None,
            },
        ]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "a.txt");
        assert_eq!(found[0].modified, Some(visited));
    }
}
