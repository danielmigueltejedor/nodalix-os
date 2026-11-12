//! Changing files: new folders, renaming, copying, moving and deleting.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::names::{unique_path, validate, NameError};

fn invalid(error: NameError) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        match error {
            NameError::Empty => "the name is empty",
            NameError::HasSlash => "a name cannot contain /",
            NameError::Reserved => "that name is reserved",
        },
    )
}

/// Makes a folder called `name` in `dir`, or `name (2)`… when that is taken.
pub fn create_folder(dir: &Path, name: &str) -> io::Result<PathBuf> {
    let name = validate(name).map_err(invalid)?;
    let path = unique_path(dir, name);
    fs::create_dir(&path)?;
    Ok(path)
}

/// Gives `path` a new name in the same folder. A name that is taken is an error, never an
/// overwrite.
pub fn rename(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    let new_name = validate(new_name).map_err(invalid)?;
    let target = path.with_file_name(new_name);
    if target == path {
        return Ok(target);
    }
    if target.symlink_metadata().is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "there is already a file with that name",
        ));
    }
    fs::rename(path, &target)?;
    Ok(target)
}

/// Copies a file, a folder with everything in it, or a link (as a link) to `target`, which must
/// not exist. A folder cannot be copied into itself.
pub fn copy(source: &Path, target: &Path) -> io::Result<()> {
    if target.starts_with(source) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a folder cannot be copied into itself",
        ));
    }
    let meta = source.symlink_metadata()?;
    if meta.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source)?, target)
    } else if meta.is_dir() {
        fs::create_dir(target)?;
        fs::set_permissions(target, meta.permissions())?;
        for child in fs::read_dir(source)? {
            let child = child?;
            copy(&child.path(), &target.join(child.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(source, target).map(|_| ())
    }
}

/// Moves `source` to `target`, which must not exist. Across drives it copies, then deletes.
pub fn move_to(source: &Path, target: &Path) -> io::Result<()> {
    if target.starts_with(source) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a folder cannot be moved into itself",
        ));
    }
    match fs::rename(source, target) {
        Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
            copy(source, target)?;
            delete(source)
        }
        other => other,
    }
}

/// Deletes for good; a link is removed without touching what it points at.
pub fn delete(path: &Path) -> io::Result<()> {
    if path.symlink_metadata()?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// What a file or folder takes up: the bytes of every regular file inside it, and how many files
/// that is. Links are counted as themselves, never followed, so a link back up the tree cannot
/// make the walk endless. Entries that cannot be read are skipped.
pub fn tree_size(path: &Path) -> (u64, u64) {
    let mut total = (0, 0);
    let mut pending = vec![path.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(meta) = next.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            if let Ok(children) = fs::read_dir(&next) {
                pending.extend(children.flatten().map(|c| c.path()));
            }
        } else {
            total.0 += meta.len();
            total.1 += 1;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archivador-ops-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn new_folders_never_collide() {
        let dir = temp("new");
        assert_eq!(
            create_folder(&dir, "Untitled").unwrap(),
            dir.join("Untitled")
        );
        assert_eq!(
            create_folder(&dir, "Untitled").unwrap(),
            dir.join("Untitled (2)")
        );
        assert!(create_folder(&dir, "").is_err());
        assert!(create_folder(&dir, "a/b").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = temp("rename");
        fs::write(dir.join("a"), "A").unwrap();
        fs::write(dir.join("b"), "B").unwrap();
        let error = rename(&dir.join("a"), "b").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(dir.join("b")).unwrap(), "B");
        assert_eq!(rename(&dir.join("a"), "c").unwrap(), dir.join("c"));
        assert!(!dir.join("a").exists());
        assert_eq!(rename(&dir.join("c"), "c").unwrap(), dir.join("c"));
        assert!(rename(&dir.join("c"), "x/y").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_folder_is_copied_whole_with_its_links() {
        let dir = temp("copy");
        fs::create_dir_all(dir.join("src/inner")).unwrap();
        fs::write(dir.join("src/inner/f.txt"), "data").unwrap();
        std::os::unix::fs::symlink("inner/f.txt", dir.join("src/link")).unwrap();
        copy(&dir.join("src"), &dir.join("dst")).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("dst/inner/f.txt")).unwrap(),
            "data"
        );
        assert!(dir
            .join("dst/link")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_link(dir.join("dst/link")).unwrap(),
            Path::new("inner/f.txt")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_folder_cannot_go_into_itself() {
        let dir = temp("self");
        fs::create_dir_all(dir.join("a")).unwrap();
        assert!(copy(&dir.join("a"), &dir.join("a/b")).is_err());
        assert!(move_to(&dir.join("a"), &dir.join("a/b")).is_err());
        assert!(dir.join("a").is_dir());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn moving_deleting_and_links() {
        let dir = temp("move");
        fs::write(dir.join("a"), "A").unwrap();
        move_to(&dir.join("a"), &dir.join("b")).unwrap();
        assert!(!dir.join("a").exists() && dir.join("b").exists());
        fs::create_dir_all(dir.join("real")).unwrap();
        fs::write(dir.join("real/keep"), "").unwrap();
        std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).unwrap();
        // Deleting a link leaves what it points at alone.
        delete(&dir.join("link")).unwrap();
        assert!(dir.join("real/keep").exists() && !dir.join("link").exists());
        delete(&dir.join("real")).unwrap();
        assert!(!dir.join("real").exists());
        assert!(delete(&dir.join("missing")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_folders_size_adds_up_what_is_inside_and_ignores_links() {
        let dir = temp("tree");
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/one"), "12345").unwrap();
        fs::write(dir.join("a/b/two"), "123").unwrap();
        std::os::unix::fs::symlink(&dir, dir.join("a/loop")).unwrap();
        let (bytes, files) = tree_size(&dir.join("a"));
        // The link counts as a file of its own, with the length of its target path.
        assert_eq!(files, 3);
        assert!(bytes >= 8);
        assert_eq!(tree_size(&dir.join("a/one")), (5, 1));
        assert_eq!(tree_size(&dir.join("missing")), (0, 0));
        fs::remove_dir_all(&dir).unwrap();
    }
}
