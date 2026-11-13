//! Thumbnails of what Archivador cannot decode itself (videos, PDFs, music with a cover…), made
//! the way every freedesktop.org desktop makes them: the programs the system lists under
//! `thumbnailers/` do the work, and the result is kept in the shared `~/.cache/thumbnails`, so a
//! thumbnail another file manager already made is reused and the ones made here serve the others.
//!
//! A thumbnailer reads files that came from anywhere, so it only ever runs inside a `bwrap`
//! sandbox: the system read-only, the one file read-only, one folder to write the picture to, no
//! network and nothing of the user's home. Without `bwrap` there are no such thumbnails.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ndl::gtk::gdk_pixbuf::{InterpType, Pixbuf};
use ndl::gtk::{gio, glib, prelude::*};
use nodalia_gtk as ndl;

/// The side of the pictures kept in the shared cache's `large` folder.
const LARGE: i32 = 256;
/// A thumbnailer that takes longer than this is stopped.
const PATIENCE: Duration = Duration::from_secs(20);
/// How many thumbnailers run at once.
const AT_ONCE: usize = 2;
/// Where failures made here are recorded, next to the ones of GNOME's own factory.
const FAILED: [&str; 2] = ["nodalix-archivador", "gnome-thumbnail-factory"];

/// One `.thumbnailer` file: what to run, and for which content types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnailer {
    exec: String,
    try_exec: Option<String>,
    mime_types: Vec<String>,
}

/// Reads a `.thumbnailer` key file. `None` when it has no command or no types.
fn parse(text: &str) -> Option<Thumbnailer> {
    let mut inside = false;
    let (mut exec, mut try_exec, mut mime_types) = (None, None, Vec::new());
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[Thumbnailer Entry]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Exec" => exec = Some(value.to_owned()),
            "TryExec" => try_exec = Some(value.to_owned()),
            "MimeType" => {
                mime_types = value
                    .split(';')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            _ => {}
        }
    }
    let exec = exec.filter(|e| !e.is_empty())?;
    (!mime_types.is_empty()).then_some(Thumbnailer {
        exec,
        try_exec,
        mime_types,
    })
}

/// The thumbnailers the system has, the user's own folder first, each file name once.
fn registry() -> &'static [Thumbnailer] {
    static REGISTRY: OnceLock<Vec<Thumbnailer>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        // Nothing can run without the sandbox, so nothing is offered without it.
        if glib::find_program_in_path("bwrap").is_none() {
            return Vec::new();
        }
        let mut dirs = vec![glib::user_data_dir()];
        dirs.extend(glib::system_data_dirs());
        let mut seen = std::collections::HashSet::new();
        let mut found = Vec::new();
        for dir in dirs {
            let Ok(files) = std::fs::read_dir(dir.join("thumbnailers")) else {
                continue;
            };
            let mut files: Vec<PathBuf> = files.flatten().map(|f| f.path()).collect();
            files.sort();
            for file in files {
                if file.extension().is_none_or(|e| e != "thumbnailer") {
                    continue;
                }
                let Some(name) = file.file_name().map(|n| n.to_owned()) else {
                    continue;
                };
                if !seen.insert(name) {
                    continue;
                }
                let Some(thumbnailer) = std::fs::read_to_string(&file).ok().and_then(|t| parse(&t))
                else {
                    continue;
                };
                let there = thumbnailer.try_exec.as_ref().is_none_or(|program| {
                    Path::new(program).is_file() || glib::find_program_in_path(program).is_some()
                });
                if there {
                    found.push(thumbnailer);
                }
            }
        }
        found
    })
}

/// The thumbnailer for files of this content type, if the system has one.
fn for_type(content_type: &str) -> Option<&'static Thumbnailer> {
    registry()
        .iter()
        .find(|t| t.mime_types.iter().any(|m| m == content_type))
}

/// Whether a thumbnail can be asked for a file of this content type.
pub fn handles(content_type: &str) -> bool {
    for_type(content_type).is_some()
}

/// The command line of a thumbnailer, with its fields filled in: `%i` the file, `%u` its URI,
/// `%o` where to write the picture, `%s` its size.
fn argv(exec: &str, input: &str, uri: &str, output: &str, size: i32) -> Option<Vec<String>> {
    let words = glib::shell_parse_argv(exec).ok()?;
    let filled: Vec<String> = words
        .into_iter()
        .map(|word| {
            let word = word.to_string_lossy().into_owned();
            let mut out = String::with_capacity(word.len());
            let mut chars = word.chars();
            while let Some(c) = chars.next() {
                if c != '%' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('i') => out.push_str(input),
                    Some('u') => out.push_str(uri),
                    Some('o') => out.push_str(output),
                    Some('s') => out.push_str(&size.to_string()),
                    Some('%') => out.push('%'),
                    Some(other) => {
                        out.push('%');
                        out.push(other);
                    }
                    None => out.push('%'),
                }
            }
            out
        })
        .collect();
    (!filled.is_empty()).then_some(filled)
}

/// The same command inside a sandbox: `/usr` and what a merged `/usr` links to it read-only, the
/// file read-only at its own path, the folder for the picture writable, and nothing else: no
/// home, no network, no other processes.
fn sandboxed(command: &[String], input: &Path, out_dir: &Path) -> Vec<String> {
    let mut args: Vec<String> = ["bwrap", "--ro-bind", "/usr", "/usr"]
        .map(str::to_owned)
        .into();
    for dir in ["/bin", "/sbin", "/lib", "/lib32", "/lib64"] {
        let path = Path::new(dir);
        match std::fs::read_link(path) {
            Ok(target) => {
                args.extend([
                    "--symlink".into(),
                    target.to_string_lossy().into_owned(),
                    dir.into(),
                ]);
            }
            Err(_) if path.is_dir() => args.extend(["--ro-bind".into(), dir.into(), dir.into()]),
            Err(_) => {}
        }
    }
    for file in [
        "/etc/ld.so.cache",
        "/etc/alternatives",
        "/etc/fonts",
        "/var/cache/fontconfig",
    ] {
        args.extend(["--ro-bind-try".into(), file.into(), file.into()]);
    }
    args.extend(
        [
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--chdir",
            "/",
            "--setenv",
            "GIO_USE_VFS",
            "local",
            "--setenv",
            "HOME",
            "/tmp",
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
        ]
        .map(str::to_owned),
    );
    let input = input.to_string_lossy().into_owned();
    let out_dir = out_dir.to_string_lossy().into_owned();
    args.extend(["--ro-bind".into(), input.clone(), input]);
    args.extend(["--bind".into(), out_dir.clone(), out_dir]);
    args.push("--".into());
    args.extend(command.iter().cloned());
    args
}

/// Lets only a few thumbnailers run at a time: a folder of films would otherwise start one each.
fn turn<T>(work: impl FnOnce() -> T) -> T {
    static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
    let (count, freed) = &RUNNING;
    {
        let mut running = count.lock().unwrap_or_else(|e| e.into_inner());
        while *running >= AT_ONCE {
            running = freed.wait(running).unwrap_or_else(|e| e.into_inner());
        }
        *running += 1;
    }
    let result = work();
    *count.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
    freed.notify_one();
    result
}

/// Runs the thumbnailer on `path` and returns the picture it made.
fn run(thumbnailer: &Thumbnailer, path: &Path, uri: &str) -> Option<Pixbuf> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let out_dir = glib::user_cache_dir()
        .join("nodalix")
        .join("archivador")
        .join("thumbnailing")
        .join(format!(
            "{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&out_dir).ok()?;
    let output = out_dir.join("thumbnail.png");
    let made = (|| {
        let command = argv(
            &thumbnailer.exec,
            &path.to_string_lossy(),
            uri,
            &output.to_string_lossy(),
            LARGE,
        )?;
        let args = sandboxed(&command, path, &out_dir);
        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < PATIENCE => {
                    std::thread::sleep(Duration::from_millis(40));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        };
        if !status.success() {
            return None;
        }
        Pixbuf::from_file(&output).ok()
    })();
    let _ = std::fs::remove_dir_all(&out_dir);
    made
}

// ----- the shared cache ------------------------------------------------------------------------

fn cache_root() -> PathBuf {
    glib::user_cache_dir().join("thumbnails")
}

/// The name a file's thumbnail has in the shared cache: the MD5 of its URI.
fn cache_name(uri: &str) -> String {
    let sum = glib::compute_checksum_for_string(glib::ChecksumType::Md5, uri)
        .map(|s| s.to_string())
        .unwrap_or_default();
    format!("{sum}.png")
}

/// Whether the picture at `file` was made for the file as it is now.
fn current(file: &Path, mtime: u64) -> Option<Pixbuf> {
    let pixbuf = Pixbuf::from_file(file).ok()?;
    let made_for = pixbuf.option("tEXt::Thumb::MTime")?;
    (made_for.as_str() == mtime.to_string()).then_some(pixbuf)
}

/// The thumbnail the shared cache has for this file, the nearest size first.
fn cached(name: &str, mtime: u64, size: i32) -> Option<Pixbuf> {
    let order: [&str; 4] = if size <= 128 {
        ["normal", "large", "x-large", "xx-large"]
    } else if size <= 256 {
        ["large", "x-large", "xx-large", "normal"]
    } else {
        ["x-large", "xx-large", "large", "normal"]
    };
    order
        .iter()
        .find_map(|dir| current(&cache_root().join(dir).join(name), mtime))
}

fn failed_before(name: &str, mtime: u64) -> bool {
    FAILED
        .iter()
        .any(|who| current(&cache_root().join("fail").join(who).join(name), mtime).is_some())
}

/// Writes a picture into the shared cache with the notes the specification asks for, in one
/// step, readable by the user alone.
fn save(pixbuf: &Pixbuf, dir: &Path, name: &str, uri: &str, mtime: u64) {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    if std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .is_err()
    {
        return;
    }
    let partial = dir.join(format!("{name}.{}.part", std::process::id()));
    let mtime = mtime.to_string();
    let saved = pixbuf.savev(
        &partial,
        "png",
        &[
            ("tEXt::Thumb::URI", uri),
            ("tEXt::Thumb::MTime", mtime.as_str()),
            ("tEXt::Software", "Nodalix Archivador"),
        ],
    );
    let placed = saved.is_ok()
        && std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(0o600)).is_ok()
        && std::fs::rename(&partial, dir.join(name)).is_ok();
    if !placed {
        let _ = std::fs::remove_file(&partial);
    }
}

/// Whether the file is somewhere a thumbnail should not be made from: the thumbnail cache
/// itself, or a place on the network, where reading a film just to draw it would be slow and
/// would cost data.
fn off_limits(path: &Path) -> bool {
    if path.starts_with(cache_root()) || path.starts_with(glib::user_runtime_dir().join("gvfs")) {
        return true;
    }
    path.parent().is_some_and(|dir| {
        gio::File::for_path(dir)
            .query_filesystem_info("filesystem::remote", gio::Cancellable::NONE)
            .is_ok_and(|info| info.boolean("filesystem::remote"))
    })
}

fn fit(pixbuf: &Pixbuf, size: i32) -> Option<Pixbuf> {
    let (w, h) = (pixbuf.width(), pixbuf.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    if w <= size && h <= size {
        return Some(pixbuf.clone());
    }
    let scale = f64::from(size) / f64::from(w.max(h));
    let (w, h) = (
        (f64::from(w) * scale).round().max(1.0) as i32,
        (f64::from(h) * scale).round().max(1.0) as i32,
    );
    pixbuf.scale_simple(w, h, InterpType::Bilinear)
}

/// The thumbnail of `path`, `size` pixels at most on a side, as a PNG: from the shared cache if
/// it has one for the file as it is now, made by the system's thumbnailer otherwise. Meant for a
/// worker thread; `None` when there is none to be had.
pub fn thumbnail(path: &Path, content_type: &str, mtime: u64, size: i32) -> Option<Vec<u8>> {
    let uri = gio::File::for_path(path).uri().to_string();
    let name = cache_name(&uri);
    let pixbuf = cached(&name, mtime, size).or_else(|| {
        let thumbnailer = for_type(content_type)?;
        if off_limits(path) || failed_before(&name, mtime) {
            return None;
        }
        match turn(|| run(thumbnailer, path, &uri)) {
            Some(made) => {
                let made = fit(&made, LARGE)?;
                save(&made, &cache_root().join("large"), &name, &uri, mtime);
                Some(made)
            }
            None => {
                // A note that it failed, so it is not tried again until the file changes.
                if let Some(blank) =
                    Pixbuf::new(ndl::gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1)
                {
                    blank.fill(0);
                    let dir = cache_root().join("fail").join(FAILED[0]);
                    save(&blank, &dir, &name, &uri, mtime);
                }
                None
            }
        }
    })?;
    fit(&pixbuf, size)?.save_to_bufferv("png", &[]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thumbnailer_file_is_read() {
        let text = "[Thumbnailer Entry]\nTryExec=ffmpegthumbnailer\nExec=ffmpegthumbnailer -i %i -o %o -s %s -f\nMimeType=video/mp4;video/x-matroska;\n";
        let t = parse(text).unwrap();
        assert_eq!(t.exec, "ffmpegthumbnailer -i %i -o %o -s %s -f");
        assert_eq!(t.try_exec.as_deref(), Some("ffmpegthumbnailer"));
        assert_eq!(t.mime_types, ["video/mp4", "video/x-matroska"]);
    }

    #[test]
    fn a_file_without_a_command_or_types_is_not_one() {
        assert_eq!(parse("[Thumbnailer Entry]\nMimeType=video/mp4;\n"), None);
        assert_eq!(parse("[Thumbnailer Entry]\nExec=x %i %o\n"), None);
        assert_eq!(parse("[Other]\nExec=x %i %o\nMimeType=a/b;\n"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn the_fields_of_a_command_are_filled_in() {
        let args = argv(
            "thumb --size=%s -i %i -u %u -o %o 100%%",
            "/home/a b/film.mp4",
            "file:///home/a%20b/film.mp4",
            "/tmp/out.png",
            256,
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "thumb",
                "--size=256",
                "-i",
                "/home/a b/film.mp4",
                "-u",
                "file:///home/a%20b/film.mp4",
                "-o",
                "/tmp/out.png",
                "100%",
            ]
        );
        // A name with spaces or quotes stays one argument: nothing is passed through a shell.
        let args = argv("t %i", "/x/a \"b\" $(c).mp4", "", "", 1).unwrap();
        assert_eq!(args, ["t", "/x/a \"b\" $(c).mp4"]);
        assert_eq!(argv("", "", "", "", 1), None);
    }

    #[test]
    fn the_sandbox_shares_only_the_file_and_the_folder_for_the_picture() {
        let command = vec!["thumb".to_owned(), "/home/u/f.pdf".to_owned()];
        let args = sandboxed(
            &command,
            Path::new("/home/u/f.pdf"),
            Path::new("/home/u/.cache/t/1"),
        );
        assert_eq!(args[0], "bwrap");
        assert!(args.contains(&"--unshare-all".to_owned()));
        assert!(args.contains(&"--die-with-parent".to_owned()));
        let split = args.iter().position(|a| a == "--").unwrap();
        assert_eq!(&args[split + 1..], &command[..]);
        let binds = |flag: &str| -> Vec<&str> {
            args[..split]
                .windows(3)
                .filter(|w| w[0] == flag)
                .map(|w| w[1].as_str())
                .collect()
        };
        // Written to: the folder for the picture, and nothing else.
        assert_eq!(binds("--bind"), ["/home/u/.cache/t/1"]);
        // Read from the home: the one file, and nothing else.
        let read: Vec<&str> = binds("--ro-bind")
            .into_iter()
            .filter(|p| p.starts_with("/home"))
            .collect();
        assert_eq!(read, ["/home/u/f.pdf"]);
    }

    #[test]
    fn a_thumbnail_is_named_after_the_md5_of_the_uri() {
        // The example of the freedesktop.org thumbnail specification.
        assert_eq!(
            cache_name("file:///home/jens/photos/me.png"),
            "c6ee772d9e49320e97ec29a7eb5b1697.png"
        );
    }

    #[test]
    fn a_saved_thumbnail_is_found_while_the_file_is_unchanged() {
        let dir = std::env::temp_dir().join(format!("archivador-thumbs-{}", std::process::id()));
        let pixbuf = Pixbuf::new(ndl::gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 300, 150).unwrap();
        pixbuf.fill(0x336699ff);
        save(&pixbuf, &dir, "x.png", "file:///x", 42);
        let file = dir.join("x.png");
        assert!(current(&file, 42).is_some());
        assert!(current(&file, 43).is_none());
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        let small = fit(&pixbuf, 100).unwrap();
        assert_eq!((small.width(), small.height()), (100, 50));
        assert_eq!(fit(&pixbuf, 400).unwrap().width(), 300);
        let _ = std::fs::remove_dir_all(dir);
    }
}
