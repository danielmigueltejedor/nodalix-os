//! The trash, as the freedesktop.org Trash specification lays it out, so that what Archivador
//! trashes shows in every other file manager and the other way round.
//!
//! Only the home trash (`$XDG_DATA_HOME/Trash`) is used. Trashing a file that lives on another
//! drive is refused rather than copied: the caller offers to delete it for good instead.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::entry::Entry;
use crate::names::unique_path;
use crate::ops;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The name inside the trash's `files` folder.
    pub name: String,
    pub original: PathBuf,
    /// `YYYY-MM-DDThh:mm:ss`, local time, as written in the `.trashinfo`.
    pub deleted: String,
}

pub struct Trash {
    files: PathBuf,
    info: PathBuf,
}

impl Trash {
    /// The trash inside `data_home` (`~/.local/share`).
    pub fn new(data_home: &Path) -> Self {
        let root = data_home.join("Trash");
        Self {
            files: root.join("files"),
            info: root.join("info"),
        }
    }

    pub fn files_dir(&self) -> &Path {
        &self.files
    }

    fn create(&self) -> io::Result<()> {
        fs::create_dir_all(&self.files)?;
        fs::create_dir_all(&self.info)
    }

    /// Moves `path` to the trash and records where it came from. `date` is the deletion time as
    /// `YYYY-MM-DDThh:mm:ss`.
    pub fn trash(&self, path: &Path, date: &str) -> io::Result<()> {
        self.create()?;
        let original = absolute(path)?;
        let name = original
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
        // The name must be free in `files` and in `info`, whichever holds a stale leftover.
        let mut target = unique_path(&self.files, name);
        while info_path(&self.info, &target).symlink_metadata().is_ok() {
            target = unique_path(&self.files, &format!("{}-", file_name(&target)));
        }
        let info = info_path(&self.info, &target);
        fs::write(
            &info,
            format!(
                "[Trash Info]\nPath={}\nDeletionDate={date}\n",
                encode_path(&original)
            ),
        )?;
        if let Err(error) = fs::rename(&original, &target) {
            let _ = fs::remove_file(&info);
            return Err(error);
        }
        Ok(())
    }

    /// What is in the trash, newest deletion first.
    pub fn list(&self) -> Vec<Item> {
        let mut items: Vec<Item> = fs::read_dir(&self.files)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let text = fs::read_to_string(info_path(&self.info, &entry.path())).ok()?;
                let (original, deleted) = parse_info(&text)?;
                Some(Item {
                    name,
                    original,
                    deleted,
                })
            })
            .collect();
        items.sort_by(|a, b| b.deleted.cmp(&a.deleted).then_with(|| a.name.cmp(&b.name)));
        items
    }

    /// The items as folder entries: named as they were before they were trashed, located where
    /// they are stored now.
    pub fn entries(&self) -> Vec<Entry> {
        self.list()
            .into_iter()
            .filter_map(|item| {
                let mut entry = Entry::at(self.files.join(&item.name))?;
                entry.name = file_name(&item.original);
                Some(entry)
            })
            .collect()
    }

    /// Puts an item back where it came from (under another name if that one is taken again),
    /// recreating its folder if needed. Returns the path it now has.
    pub fn restore(&self, name: &str) -> io::Result<PathBuf> {
        let item = self
            .list()
            .into_iter()
            .find(|i| i.name == name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not in the trash"))?;
        let dir = item
            .original
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no folder"))?;
        fs::create_dir_all(dir)?;
        let target = unique_path(dir, &file_name(&item.original));
        let stored = self.files.join(name);
        ops::move_to(&stored, &target)?;
        let _ = fs::remove_file(info_path(&self.info, &stored));
        Ok(target)
    }

    /// Deletes one item for good.
    pub fn delete(&self, name: &str) -> io::Result<()> {
        let stored = self.files.join(name);
        ops::delete(&stored)?;
        let _ = fs::remove_file(info_path(&self.info, &stored));
        Ok(())
    }

    /// Deletes everything in the trash, and any record left without its file.
    pub fn empty(&self) -> io::Result<()> {
        for item in self.list() {
            self.delete(&item.name)?;
        }
        for leftover in fs::read_dir(&self.files).into_iter().flatten().flatten() {
            ops::delete(&leftover.path())?;
        }
        for leftover in fs::read_dir(&self.info).into_iter().flatten().flatten() {
            let _ = fs::remove_file(leftover.path());
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        fs::read_dir(&self.files).map_or(true, |mut d| d.next().is_none())
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn info_path(info_dir: &Path, stored: &Path) -> PathBuf {
    info_dir.join(format!("{}.trashinfo", file_name(stored)))
}

fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

/// The spec's `Path=` value: the path with every byte but unreserved ones and `/` as `%XX`.
pub(crate) fn encode_path(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::new();
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub(crate) fn decode_path(text: &str) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (
            bytes[i],
            bytes.get(i + 1).and_then(|&b| hex(b)),
            bytes.get(i + 2).and_then(|&b| hex(b)),
        ) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                i += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    PathBuf::from(std::ffi::OsString::from_vec(out))
}

fn parse_info(text: &str) -> Option<(PathBuf, String)> {
    let mut path = None;
    let mut date = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("Path=") {
            path = Some(decode_path(value.trim()));
        } else if let Some(value) = line.strip_prefix("DeletionDate=") {
            date = Some(value.trim().to_owned());
        }
    }
    Some((path?, date.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archivador-trash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn paths_round_trip_through_the_spec_encoding() {
        let path = Path::new("/home/a/My files/ñandú #1 100%.txt");
        let encoded = encode_path(path);
        assert_eq!(
            encoded,
            "/home/a/My%20files/%C3%B1and%C3%BA%20%231%20100%25.txt"
        );
        assert_eq!(decode_path(&encoded), path);
        // A stray percent sign that is not an escape stays as it is.
        assert_eq!(decode_path("/a/100%"), Path::new("/a/100%"));
    }

    #[test]
    fn a_trashed_file_is_listed_and_can_be_restored() {
        let dir = temp("restore");
        let trash = Trash::new(&dir.join("share"));
        assert!(trash.is_empty());
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("docs/a b.txt"), "A").unwrap();
        trash
            .trash(&dir.join("docs/a b.txt"), "2026-11-12T10:00:00")
            .unwrap();
        assert!(!dir.join("docs/a b.txt").exists());
        assert!(!trash.is_empty());
        let items = trash.list();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "a b.txt");
        assert_eq!(items[0].original, dir.join("docs/a b.txt"));
        assert_eq!(items[0].deleted, "2026-11-12T10:00:00");
        let entries = trash.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a b.txt");
        assert_eq!(entries[0].path, dir.join("share/Trash/files/a b.txt"));
        assert_eq!(entries[0].size, 1);
        let info = fs::read_to_string(dir.join("share/Trash/info/a b.txt.trashinfo")).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath="));
        assert_eq!(trash.restore("a b.txt").unwrap(), dir.join("docs/a b.txt"));
        assert_eq!(fs::read_to_string(dir.join("docs/a b.txt")).unwrap(), "A");
        assert!(trash.is_empty() && trash.list().is_empty());
        assert!(!dir.join("share/Trash/info/a b.txt.trashinfo").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn two_files_with_one_name_stay_apart_and_restoring_never_overwrites() {
        let dir = temp("twins");
        let trash = Trash::new(&dir.join("share"));
        fs::create_dir_all(dir.join("x")).unwrap();
        fs::create_dir_all(dir.join("y")).unwrap();
        fs::write(dir.join("x/n.txt"), "X").unwrap();
        fs::write(dir.join("y/n.txt"), "Y").unwrap();
        trash
            .trash(&dir.join("x/n.txt"), "2026-01-01T00:00:00")
            .unwrap();
        trash
            .trash(&dir.join("y/n.txt"), "2026-01-02T00:00:00")
            .unwrap();
        let items = trash.list();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].original, dir.join("y/n.txt"), "newest first");
        fs::write(dir.join("x/n.txt"), "new").unwrap();
        let restored = trash.restore(&items[1].name).unwrap();
        assert_eq!(restored, dir.join("x/n (2).txt"));
        assert_eq!(fs::read_to_string(dir.join("x/n.txt")).unwrap(), "new");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn restoring_recreates_a_folder_that_was_deleted() {
        let dir = temp("recreate");
        let trash = Trash::new(&dir.join("share"));
        fs::create_dir_all(dir.join("gone")).unwrap();
        fs::write(dir.join("gone/f"), "F").unwrap();
        trash
            .trash(&dir.join("gone/f"), "2026-01-01T00:00:00")
            .unwrap();
        fs::remove_dir(dir.join("gone")).unwrap();
        trash.restore("f").unwrap();
        assert!(dir.join("gone/f").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn emptying_removes_files_folders_and_records() {
        let dir = temp("empty");
        let trash = Trash::new(&dir.join("share"));
        fs::create_dir_all(dir.join("d/sub")).unwrap();
        fs::write(dir.join("d/sub/f"), "").unwrap();
        fs::write(dir.join("one"), "").unwrap();
        trash.trash(&dir.join("d"), "2026-01-01T00:00:00").unwrap();
        trash
            .trash(&dir.join("one"), "2026-01-01T00:00:01")
            .unwrap();
        trash.empty().unwrap();
        assert!(trash.is_empty());
        assert_eq!(
            fs::read_dir(dir.join("share/Trash/info")).unwrap().count(),
            0
        );
        assert!(trash.restore("one").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_move_leaves_no_record_behind() {
        let dir = temp("fail");
        let trash = Trash::new(&dir.join("share"));
        assert!(trash
            .trash(&dir.join("missing"), "2026-01-01T00:00:00")
            .is_err());
        assert_eq!(
            fs::read_dir(dir.join("share/Trash/info")).unwrap().count(),
            0
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
