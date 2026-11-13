//! What Archivador is asked to show from outside: the command line it is started with (the one
//! Nautilus took, which other programs still use), and the URIs of the freedesktop.org
//! `org.freedesktop.FileManager1` interface that browsers and chat apps call for "Show in folder".

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

/// A place that can be asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Path(PathBuf),
    Trash,
    Recent,
}

/// What `%XX` escapes stand for, as raw bytes: a file name need not be UTF-8.
fn unescape(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}

/// The place a URI or path names. Relative paths are taken from `cwd`. URIs of a kind that is
/// not a local file (`https:`, `smb:` before it is mounted…) name nothing.
pub fn target(arg: &str, cwd: &Path) -> Option<Target> {
    if arg.is_empty() {
        return None;
    }
    let lower = arg.to_ascii_lowercase();
    if lower.starts_with("trash:") {
        return Some(Target::Trash);
    }
    if lower.starts_with("recent:") {
        return Some(Target::Recent);
    }
    if lower.starts_with("file:") {
        let rest = &arg["file:".len()..];
        // `file:///path` and `file://localhost/path`; another host is not this computer.
        let path = if let Some(after) = rest.strip_prefix("//") {
            let (host, path) = after.split_at(after.find('/')?);
            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                return None;
            }
            path
        } else {
            rest
        };
        // A fragment or query is not part of a file's name.
        let path = path.split(['?', '#']).next().unwrap_or(path);
        if !path.starts_with('/') {
            return None;
        }
        let bytes = unescape(path)?;
        return Some(Target::Path(clean(&PathBuf::from(OsString::from_vec(
            bytes,
        )))));
    }
    // Something like `https://…` or `smb://…`: a URI, but not a file here.
    if let Some(colon) = arg.find(':') {
        let scheme = &arg[..colon];
        if colon > 1
            && arg[colon..].starts_with("://")
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        {
            return None;
        }
    }
    let path = Path::new(arg);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    Some(Target::Path(clean(&path)))
}

/// The path with `.` and `..` worked out and doubled slashes gone, without touching the disk.
fn clean(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                if out.parent().is_some() {
                    out.pop();
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        out
    }
}

/// Items to show, gathered by the folder that holds them, in the order they were asked for:
/// one window per folder, with its items selected. The root has no folder above it, so it is
/// shown itself.
pub fn by_folder(items: &[PathBuf]) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let mut groups: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
    for item in items {
        let (folder, selected) = match item.parent() {
            Some(parent) => (parent.to_path_buf(), Some(item.clone())),
            None => (item.clone(), None),
        };
        let group = match groups.iter().position(|(f, _)| *f == folder) {
            Some(i) => &mut groups[i].1,
            None => {
                groups.push((folder, Vec::new()));
                &mut groups.last_mut().expect("just pushed").1
            }
        };
        if let Some(selected) = selected {
            if !group.contains(&selected) {
                group.push(selected);
            }
        }
    }
    groups
}

/// The command line, as Nautilus took it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    /// `-w`, `--new-window`: a window even when one is open already.
    pub new_window: bool,
    /// `-q`, `--quit`: the running Archivador quits.
    pub quit: bool,
    /// `-s`, `--select`: the items to show selected in the folder that holds them.
    pub select: Vec<String>,
    /// Folders to open; a file opens its folder with the file selected.
    pub locations: Vec<String>,
    /// Options that are not known, to be mentioned and otherwise left alone.
    pub unknown: Vec<String>,
}

/// Reads the arguments that follow the program's name.
pub fn parse(args: &[String]) -> Command {
    let mut command = Command::default();
    let mut args = args.iter();
    let mut only_locations = false;
    while let Some(arg) = args.next() {
        if only_locations || !arg.starts_with('-') || arg == "-" {
            command.locations.push(arg.clone());
            continue;
        }
        match arg.as_str() {
            "--" => only_locations = true,
            "-w" | "--new-window" => command.new_window = true,
            "-q" | "--quit" => command.quit = true,
            "-s" | "--select" => {
                // Nautilus's `--select` takes the items that follow it.
                command.select.extend(args.by_ref().cloned());
            }
            _ if arg.starts_with("--select=") => {
                command.select.push(arg["--select=".len()..].to_owned());
            }
            // GApplication's own options, when it has not taken them already.
            _ if arg.starts_with("--gapplication-") => {}
            _ => command.unknown.push(arg.clone()),
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(p: &str) -> Option<Target> {
        Some(Target::Path(PathBuf::from(p)))
    }

    #[test]
    fn file_uris_become_paths() {
        let cwd = Path::new("/home/ana");
        assert_eq!(
            target("file:///home/ana/Fotos", cwd),
            path("/home/ana/Fotos")
        );
        assert_eq!(target("FILE:///tmp/x", cwd), path("/tmp/x"));
        assert_eq!(
            target("file:///home/ana/Mis%20cosas/caf%C3%A9.txt", cwd),
            path("/home/ana/Mis cosas/café.txt")
        );
        assert_eq!(
            target("file://localhost/etc/hosts", cwd),
            path("/etc/hosts")
        );
        assert_eq!(target("file:/tmp/one-slash", cwd), path("/tmp/one-slash"));
        assert_eq!(target("file:///tmp/a.pdf#page=2", cwd), path("/tmp/a.pdf"));
        assert_eq!(
            target("file:///tmp/../etc/./hosts", cwd),
            path("/etc/hosts")
        );
        assert_eq!(target("file:///", cwd), path("/"));
    }

    #[test]
    fn a_name_that_is_not_utf8_survives() {
        let Some(Target::Path(p)) = target("file:///tmp/%FFname", Path::new("/")) else {
            panic!("no path");
        };
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(p.as_os_str().as_bytes(), b"/tmp/\xffname");
    }

    #[test]
    fn what_is_not_a_local_file_names_nothing() {
        let cwd = Path::new("/home/ana");
        assert_eq!(target("file://otherhost/share/x", cwd), None);
        assert_eq!(target("file:///tmp/bad%zzescape", cwd), None);
        assert_eq!(target("file:///tmp/cut%2", cwd), None);
        assert_eq!(target("https://example.org/a", cwd), None);
        assert_eq!(target("smb://server/share", cwd), None);
        assert_eq!(target("", cwd), None);
        assert_eq!(target("file:relative", cwd), None);
    }

    #[test]
    fn virtual_places_and_plain_paths() {
        let cwd = Path::new("/home/ana/Proyectos");
        assert_eq!(target("trash:///", cwd), Some(Target::Trash));
        assert_eq!(target("recent:///", cwd), Some(Target::Recent));
        assert_eq!(target("/var/log", cwd), path("/var/log"));
        assert_eq!(
            target("notas.txt", cwd),
            path("/home/ana/Proyectos/notas.txt")
        );
        assert_eq!(target("../Fotos/", cwd), path("/home/ana/Fotos"));
        assert_eq!(target(".", cwd), path("/home/ana/Proyectos"));
        // A colon in a file name is not a URI.
        assert_eq!(target("a:b.txt", cwd), path("/home/ana/Proyectos/a:b.txt"));
        assert_eq!(target("C:/x", cwd), path("/home/ana/Proyectos/C:/x"));
    }

    #[test]
    fn items_are_shown_in_their_folders() {
        let items: Vec<PathBuf> = ["/a/x", "/b/y", "/a/z", "/a/x", "/"]
            .iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(
            by_folder(&items),
            [
                (
                    PathBuf::from("/a"),
                    vec![PathBuf::from("/a/x"), PathBuf::from("/a/z")]
                ),
                (PathBuf::from("/b"), vec![PathBuf::from("/b/y")]),
                (PathBuf::from("/"), vec![]),
            ]
        );
        assert!(by_folder(&[]).is_empty());
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_command_line_nautilus_took() {
        assert_eq!(parse(&[]), Command::default());
        let c = parse(&args(&["-w", "/tmp", "file:///home"]));
        assert!(c.new_window && !c.quit);
        assert_eq!(c.locations, ["/tmp", "file:///home"]);
        let c = parse(&args(&["--new-window", "--quit"]));
        assert!(c.new_window && c.quit);
        assert!(parse(&args(&["-q"])).quit);
        let c = parse(&args(&["--select", "/a/b.txt", "/c/d"]));
        assert_eq!(c.select, ["/a/b.txt", "/c/d"]);
        assert!(c.locations.is_empty());
        let c = parse(&args(&["/x", "-s", "/y"]));
        assert_eq!((c.locations, c.select), (args(&["/x"]), args(&["/y"])));
        let c = parse(&args(&["--select=/a b", "--", "-not-an-option"]));
        assert_eq!(c.select, ["/a b"]);
        assert_eq!(c.locations, ["-not-an-option"]);
        let c = parse(&args(&["--gapplication-service", "--geometry=1x1", "-"]));
        assert_eq!(c.unknown, ["--geometry=1x1"]);
        assert_eq!(c.locations, ["-"]);
    }
}
