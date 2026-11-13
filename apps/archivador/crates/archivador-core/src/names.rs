//! Names: how folders are ordered and how a name that is taken is made unique.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Orders names the way people read them: case does not matter and runs of digits compare as
/// numbers, so `file2` comes before `file10`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (nx, ny) = (take_number(&mut a), take_number(&mut b));
                // Compare by value, then by how many zeros were written.
                let order = nx
                    .trim_start_matches('0')
                    .len()
                    .cmp(&ny.trim_start_matches('0').len())
                    .then_with(|| nx.trim_start_matches('0').cmp(ny.trim_start_matches('0')))
                    .then_with(|| nx.len().cmp(&ny.len()));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut digits = String::new();
    while let Some(c) = chars.peek().copied().filter(char::is_ascii_digit) {
        digits.push(c);
        chars.next();
    }
    digits
}

/// Why a name cannot be given to a file.
#[derive(Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    HasSlash,
    Reserved,
}

/// Checks a name typed by the user (the rename and new-folder boxes).
pub fn validate(name: &str) -> Result<&str, NameError> {
    let name = name.trim();
    if name.is_empty() {
        Err(NameError::Empty)
    } else if name.contains('/') {
        Err(NameError::HasSlash)
    } else if name == "." || name == ".." {
        Err(NameError::Reserved)
    } else {
        Ok(name)
    }
}

/// A path in `dir` for `name` that does not exist yet: `name`, then `name (2)`, `name (3)`…
/// The counter goes before the extension, so `photo.png` becomes `photo (2).png`.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if first.symlink_metadata().is_err() {
        return first;
    }
    let (stem, ext) = split_extension(name);
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| p.symlink_metadata().is_err())
        .expect("an unbounded counter always finds a free name")
}

/// `("archive.tar", ".gz")`, `("photo", ".png")`, `(".bashrc", "")`, `("README", "")`.
fn split_extension(name: &str) -> (&str, &str) {
    // A double extension of a compressed tarball stays whole: `a.tar.gz` -> `a`, `.tar.gz`.
    for double in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if let Some(stem) = name.strip_suffix(double) {
            if !stem.is_empty() {
                return (stem, &name[stem.len()..]);
            }
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_compare_as_numbers_and_case_is_ignored() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("File", "file"), Ordering::Equal);
        assert_eq!(natural_cmp("b", "A"), Ordering::Greater);
        assert_eq!(natural_cmp("a", "a1"), Ordering::Less);
        assert_eq!(natural_cmp("img007", "img7"), Ordering::Greater);
        assert_eq!(natural_cmp("", "a"), Ordering::Less);
    }

    #[test]
    fn natural_order_sorts_a_listing() {
        let mut names = vec!["10.txt", "2.txt", "b", "A", "1.txt"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, ["1.txt", "2.txt", "10.txt", "A", "b"]);
    }

    #[test]
    fn names_typed_by_the_user_are_checked() {
        assert_eq!(validate("  notes "), Ok("notes"));
        assert_eq!(validate(""), Err(NameError::Empty));
        assert_eq!(validate("   "), Err(NameError::Empty));
        assert_eq!(validate("a/b"), Err(NameError::HasSlash));
        assert_eq!(validate(".."), Err(NameError::Reserved));
        assert_eq!(validate(".hidden"), Ok(".hidden"));
    }

    #[test]
    fn extensions_split_where_people_expect() {
        assert_eq!(split_extension("photo.png"), ("photo", ".png"));
        assert_eq!(split_extension("a.tar.gz"), ("a", ".tar.gz"));
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension("README"), ("README", ""));
        assert_eq!(split_extension("x.y.z"), ("x.y", ".z"));
    }

    #[test]
    fn a_taken_name_gets_a_counter_before_the_extension() {
        let dir = std::env::temp_dir().join(format!("archivador-names-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_path(&dir, "photo.png"), dir.join("photo.png"));
        std::fs::write(dir.join("photo.png"), "").unwrap();
        assert_eq!(unique_path(&dir, "photo.png"), dir.join("photo (2).png"));
        std::fs::write(dir.join("photo (2).png"), "").unwrap();
        assert_eq!(unique_path(&dir, "photo.png"), dir.join("photo (3).png"));
        // A broken symlink is still a name in use.
        std::os::unix::fs::symlink("/nonexistent", dir.join("dangling")).unwrap();
        assert_eq!(unique_path(&dir, "dangling"), dir.join("dangling (2)"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
