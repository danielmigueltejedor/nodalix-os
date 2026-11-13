//! The Recent place: the files the user opened lately, from wherever they are.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::entry::Entry;

/// What the desktop remembers about one recent file.
pub struct Item {
    pub path: PathBuf,
    pub visited: Option<SystemTime>,
}

/// How many recent files are shown. The desktop's list can hold thousands, each one costs a
/// look at the disk, and a window that stats them all before it draws anything seems stuck.
pub const LIMIT: usize = 200;

/// The files that are still there, newest visit first, as entries. Each one's time is when it was
/// last opened, not when it was last written, so sorting by it puts the newest visit first. Only
/// the newest [`LIMIT`] are looked at.
pub fn entries(items: impl IntoIterator<Item = Item>) -> Vec<Entry> {
    let mut items: Vec<Item> = items.into_iter().collect();
    items.sort_by(|a, b| b.visited.cmp(&a.visited));
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if out.len() >= LIMIT {
            break;
        }
        if !seen.insert(item.path.clone()) {
            continue;
        }
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

    #[test]
    fn only_the_newest_are_looked_at_and_they_come_newest_first() {
        let dir =
            std::env::temp_dir().join(format!("archivador-recent-many-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let items: Vec<Item> = (0..LIMIT as u64 + 20)
            .map(|i| {
                let path = dir.join(format!("{i}.txt"));
                std::fs::write(&path, "x").unwrap();
                Item {
                    path,
                    visited: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(i)),
                }
            })
            .collect();
        let found = entries(items);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found.len(), LIMIT);
        assert_eq!(found[0].name, format!("{}.txt", LIMIT + 19));
        assert!(found.windows(2).all(|w| w[0].modified >= w[1].modified));
    }
}
