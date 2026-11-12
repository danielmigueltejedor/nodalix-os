//! Copy, cut and paste inside Archivador.

use std::io;
use std::path::{Path, PathBuf};

use crate::names::unique_path;
use crate::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Cut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clipboard {
    pub mode: Mode,
    pub paths: Vec<PathBuf>,
}

impl Clipboard {
    pub fn new(mode: Mode, paths: Vec<PathBuf>) -> Self {
        Self { mode, paths }
    }

    /// Puts the files in `dir`. A copy keeps both; a cut moves, and moving a file into the folder
    /// it is already in changes nothing. A name that is taken becomes `name (2)`. Returns where
    /// each pasted file ended up; it stops at the first error.
    pub fn paste(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut pasted = Vec::new();
        for source in &self.paths {
            let Some(name) = source.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if self.mode == Mode::Cut && source.parent() == Some(dir) {
                pasted.push(source.clone());
                continue;
            }
            let target = unique_path(dir, name);
            match self.mode {
                Mode::Copy => ops::copy(source, &target)?,
                Mode::Cut => ops::move_to(source, &target)?,
            }
            pasted.push(target);
        }
        Ok(pasted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archivador-clip-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("one")).unwrap();
        fs::create_dir_all(dir.join("two")).unwrap();
        dir
    }

    #[test]
    fn copying_twice_into_one_folder_keeps_every_copy() {
        let dir = temp("copy");
        fs::write(dir.join("one/a.txt"), "A").unwrap();
        let clip = Clipboard::new(Mode::Copy, vec![dir.join("one/a.txt")]);
        let first = clip.paste(&dir.join("two")).unwrap();
        let second = clip.paste(&dir.join("two")).unwrap();
        assert_eq!(first, [dir.join("two/a.txt")]);
        assert_eq!(second, [dir.join("two/a (2).txt")]);
        assert!(dir.join("one/a.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cutting_moves_and_cutting_into_the_same_folder_is_a_no_op() {
        let dir = temp("cut");
        fs::write(dir.join("one/a.txt"), "A").unwrap();
        let clip = Clipboard::new(Mode::Cut, vec![dir.join("one/a.txt")]);
        assert_eq!(
            clip.paste(&dir.join("one")).unwrap(),
            [dir.join("one/a.txt")]
        );
        assert!(dir.join("one/a.txt").exists());
        assert_eq!(
            clip.paste(&dir.join("two")).unwrap(),
            [dir.join("two/a.txt")]
        );
        assert!(!dir.join("one/a.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_source_is_an_error_and_stops_the_paste() {
        let dir = temp("missing");
        fs::write(dir.join("one/b.txt"), "B").unwrap();
        let clip = Clipboard::new(
            Mode::Copy,
            vec![dir.join("one/gone.txt"), dir.join("one/b.txt")],
        );
        assert!(clip.paste(&dir.join("two")).is_err());
        assert!(!dir.join("two/b.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
