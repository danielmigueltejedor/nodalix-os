use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct SpecialDirs {
    pub home: String,
    pub desktop: Option<String>,
    pub downloads: Option<String>,
    pub documents: Option<String>,
    pub pictures: Option<String>,
    pub videos: Option<String>,
    pub music: Option<String>,
    pub data: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PathProperties {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub size_display: String,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub permissions: String,
    pub is_symlink: bool,
    pub item_count: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct OpenWithApp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct StartupBundle {
    pub home: String,
    pub platform: crate::platform::PlatformInfo,
    pub sidebar_items: Vec<crate::sidebar::SidebarItem>,
    pub localsend_available: bool,
    pub folder_customizations: std::collections::HashMap<String, crate::folder_customization::FolderStyle>,
}

#[derive(Clone, Copy)]
enum ClipOp {
    Copy,
    Cut,
}

struct Clipboard {
    op: ClipOp,
    paths: Vec<String>,
}

static SPECIAL_DIRS: OnceLock<Result<SpecialDirs, String>> = OnceLock::new();
static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

fn err(msg: impl Into<String>) -> String {
    msg.into()
}

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_string())
}

fn expand_user_dir(value: &str, home: &Path) -> PathBuf {
    PathBuf::from(value.replace("$HOME", &home.to_string_lossy()))
}

fn read_user_dirs_entry(home: &Path, key: &str) -> Option<PathBuf> {
    let config = home.join(".config/user-dirs.dirs");
    let content = fs::read_to_string(config).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        if k.trim() != key {
            continue;
        }
        let raw = v.trim().trim_matches('"');
        return Some(expand_user_dir(raw, home));
    }
    None
}

fn resolve_user_dir(
    home: &Path,
    xdg_env: &str,
    user_dirs_key: &str,
    fallback_name: &str,
) -> Option<String> {
    if let Ok(raw) = std::env::var(xdg_env) {
        let path = expand_user_dir(&raw, home);
        if path.is_dir() {
            return Some(path_to_string_fast(&path));
        }
    }
    if let Some(path) = read_user_dirs_entry(home, user_dirs_key) {
        if path.is_dir() {
            return Some(path_to_string_fast(&path));
        }
    }
    let fallback = home.join(fallback_name);
    if fallback.is_dir() {
        return Some(path_to_string_fast(&fallback));
    }
    None
}

fn path_to_string_fast(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    if bytes < 1024 * 1024 * 1024 {
        return format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0);
    }
    format!("{:.1} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
}

fn compute_special_dirs() -> Result<SpecialDirs, String> {
    crate::platform::debug_log("compute_special_dirs");
    let home = home_dir()?;
    let home_str = path_to_string_fast(&home);
    Ok(SpecialDirs {
        home: home_str,
        desktop: resolve_user_dir(&home, "XDG_DESKTOP_DIR", "XDG_DESKTOP_DIR", "Desktop"),
        downloads: resolve_user_dir(&home, "XDG_DOWNLOAD_DIR", "XDG_DOWNLOAD_DIR", "Downloads"),
        documents: resolve_user_dir(
            &home,
            "XDG_DOCUMENTS_DIR",
            "XDG_DOCUMENTS_DIR",
            "Documents",
        ),
        pictures: resolve_user_dir(&home, "XDG_PICTURES_DIR", "XDG_PICTURES_DIR", "Pictures"),
        videos: resolve_user_dir(&home, "XDG_VIDEOS_DIR", "XDG_VIDEOS_DIR", "Videos"),
        music: resolve_user_dir(&home, "XDG_MUSIC_DIR", "XDG_MUSIC_DIR", "Music"),
        data: if Path::new("/data").is_dir() {
            Some("/data".into())
        } else {
            None
        },
    })
}

pub fn get_special_dirs_cached() -> Result<SpecialDirs, String> {
    SPECIAL_DIRS
        .get_or_init(compute_special_dirs)
        .clone()
}

#[tauri::command]
pub fn get_special_dirs() -> Result<SpecialDirs, String> {
    get_special_dirs_cached()
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    crate::platform::debug_log(&format!("list_directory {path}"));
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(err(format!("Not a directory: {path}")));
    }

    let read_dir = fs::read_dir(&dir).map_err(|e| format!("Cannot read {path}: {e}"))?;
    let mut entries = Vec::new();

    for item in read_dir {
        let item = match item {
            Ok(i) => i,
            Err(_) => continue,
        };
        let file_name = item.file_name().to_string_lossy().into_owned();
        if file_name == "." || file_name == ".." {
            continue;
        }

        let file_type = match item.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let is_dir = file_type.is_dir();
        let file_path = item.path();
        let full_path = path_to_string_fast(&file_path);

        let size = if is_dir {
            0
        } else {
            item.metadata().map(|m| m.len()).unwrap_or(0)
        };

        entries.push(FileEntry {
            name: file_name,
            path: full_path,
            is_dir,
            size,
            modified: None,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(err(format!("Path does not exist: {path}")));
    }
    Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to launch xdg-open: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_with(path: String, app_id: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(err(format!("Path does not exist: {path}")));
    }
    if app_id == "xdg-default" {
        return open_path(path);
    }
    Command::new("gtk-launch")
        .arg(&app_id)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("gtk-launch failed: {e}. Try xdg-default."))?;
    Ok(())
}

#[tauri::command]
pub fn get_open_with_apps(path: String) -> Result<Vec<OpenWithApp>, String> {
    let _ = path;
    Ok(vec![
        OpenWithApp {
            id: "xdg-default".into(),
            name: "Default application".into(),
        },
        OpenWithApp {
            id: "org.gnome.TextEditor".into(),
            name: "Text Editor".into(),
        },
        OpenWithApp {
            id: "code".into(),
            name: "Visual Studio Code".into(),
        },
        OpenWithApp {
            id: "org.gnome.eog".into(),
            name: "Image Viewer".into(),
        },
        OpenWithApp {
            id: "vlc".into(),
            name: "VLC".into(),
        },
    ])
}

#[tauri::command]
pub fn create_folder(parent: String, name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(err("Folder name cannot be empty"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(err("Folder name cannot contain path separators"));
    }
    let parent_path = PathBuf::from(&parent);
    if !parent_path.is_dir() {
        return Err(err(format!("Parent is not a directory: {parent}")));
    }
    let new_path = parent_path.join(trimmed);
    if new_path.exists() {
        return Err(err(format!("Already exists: {}", new_path.display())));
    }
    fs::create_dir(&new_path).map_err(|e| e.to_string())?;
    Ok(path_to_string_fast(&new_path))
}

#[tauri::command]
pub fn rename_path(old_path: String, new_name: String) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(err("New name cannot be empty"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(err("New name cannot contain path separators"));
    }
    let source = PathBuf::from(&old_path);
    if !source.exists() {
        return Err(err(format!("Path does not exist: {old_path}")));
    }
    let parent = source
        .parent()
        .ok_or_else(|| err("Cannot resolve parent directory"))?;
    let dest = parent.join(trimmed);
    if dest.exists() {
        return Err(err(format!("Already exists: {}", dest.display())));
    }
    fs::rename(&source, &dest).map_err(|e| e.to_string())?;
    Ok(path_to_string_fast(&dest))
}

#[tauri::command]
pub fn trash_path(path: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(err(format!("Path does not exist: {path}")));
    }
    match Command::new("gio").args(["trash", &path]).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(err(format!("gio trash failed (exit {status})"))),
        Err(e) => Err(err(format!(
            "gio trash unavailable ({e}). Install glib2 (gio)."
        ))),
    }
}

#[tauri::command]
pub fn trash_paths(paths: Vec<String>) -> Result<(), String> {
    for path in paths {
        trash_path(path)?;
    }
    Ok(())
}

#[tauri::command]
pub fn copy_paths(paths: Vec<String>) -> Result<(), String> {
    let mut clip = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    *clip = Some(Clipboard {
        op: ClipOp::Copy,
        paths,
    });
    Ok(())
}

#[tauri::command]
pub fn cut_paths(paths: Vec<String>) -> Result<(), String> {
    let mut clip = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    *clip = Some(Clipboard {
        op: ClipOp::Cut,
        paths,
    });
    Ok(())
}

#[tauri::command]
pub fn clipboard_has_content() -> bool {
    CLIPBOARD.lock().ok().and_then(|c| c.as_ref()).is_some()
}

fn copy_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            copy_recursive(&entry.path(), &dst.join(name))?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

#[tauri::command]
pub fn paste_into(target_dir: String) -> Result<(), String> {
    let target = PathBuf::from(&target_dir);
    if !target.is_dir() {
        return Err(err("Paste target must be a directory"));
    }
    let guard = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    let clip = guard.as_ref().ok_or_else(|| err("Nothing to paste"))?;
    let paths = clip.paths.clone();
    let op = match clip.op {
        ClipOp::Copy => ClipOp::Copy,
        ClipOp::Cut => ClipOp::Cut,
    };

    for src_str in &paths {
        let src = PathBuf::from(src_str);
        if !src.exists() {
            continue;
        }
        let name = src
            .file_name()
            .ok_or_else(|| err("Invalid source path"))?
            .to_owned();
        let mut dest = target.join(&name);
        if dest.exists() {
            dest = unique_path(&dest);
        }
        match op {
            ClipOp::Copy => copy_recursive(&src, &dest).map_err(|e| e.to_string())?,
            ClipOp::Cut => fs::rename(&src, &dest).map_err(|e| e.to_string())?,
        }
    }

    if matches!(op, ClipOp::Cut) {
        *CLIPBOARD.lock().map_err(|_| err("clipboard lock"))? = None;
    }
    Ok(())
}

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "copy".into());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(Path::new("."));
    for i in 1..1000 {
        let candidate = parent.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

#[tauri::command]
pub fn move_to(paths: Vec<String>, target_dir: String) -> Result<(), String> {
    let target = PathBuf::from(&target_dir);
    if !target.is_dir() {
        return Err(err("Target must be a directory"));
    }
    for src_str in paths {
        let src = PathBuf::from(&src_str);
        if !src.exists() {
            return Err(err(format!("Path does not exist: {src_str}")));
        }
        let name = src.file_name().ok_or_else(|| err("Invalid path"))?;
        let dest = target.join(name);
        if dest.exists() {
            return Err(err(format!("Already exists: {}", dest.display())));
        }
        fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ts_secs(path: &Path, field: fn(&fs::Metadata) -> io::Result<std::time::SystemTime>) -> Option<u64> {
    let meta = fs::symlink_metadata(path).ok()?;
    field(&meta)
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn count_dir_items(path: &Path) -> Option<u64> {
    const MAX: u64 = 5000;
    let read = fs::read_dir(path).ok()?;
    let mut count = 0u64;
    for entry in read {
        if entry.is_ok() {
            count += 1;
            if count >= MAX {
                return Some(MAX);
            }
        }
    }
    Some(count)
}

#[tauri::command]
pub fn get_properties(path: String) -> Result<PathProperties, String> {
    let p = PathBuf::from(&path);
    let meta = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
    let is_symlink = meta.file_type().is_symlink();
    let is_dir = if is_symlink {
        fs::metadata(&p).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        meta.is_dir()
    };

    let size = if is_dir {
        0
    } else {
        meta.len()
    };

    let kind = if is_dir {
        "Folder".into()
    } else if is_symlink {
        "Symlink".into()
    } else {
        "File".into()
    };

    let mode = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", meta.permissions().mode() & 0o777)
        }
        #[cfg(not(unix))]
        {
            "—".into()
        }
    };

    Ok(PathProperties {
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path_to_string_fast(&p),
        kind,
        size,
        size_display: if is_dir {
            "—".into()
        } else {
            format_size(size)
        },
        created: ts_secs(&p, |m| m.created()),
        modified: ts_secs(&p, |m| m.modified()),
        permissions: mode,
        is_symlink,
        item_count: if is_dir { count_dir_items(&p) } else { None },
    })
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn open_terminal_here(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(err("Not a directory"));
    }
    if which("foot") {
        Command::new("foot")
            .args(["-D", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("kitty") {
        Command::new("kitty")
            .args(["--directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("alacritty") {
        Command::new("alacritty")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("ghostty") {
        Command::new("ghostty")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("wezterm") {
        Command::new("wezterm")
            .args(["start", "--cwd", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("gnome-terminal") {
        Command::new("gnome-terminal")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err(err("No supported terminal found"))
}

#[tauri::command]
pub fn startup_bundle() -> Result<StartupBundle, String> {
    crate::platform::debug_log("startup_bundle");
    let dirs = get_special_dirs_cached()?;
    let home = dirs.home.clone();
    let platform = crate::platform::get_platform_info();
    let sidebar_items = crate::sidebar::build_sidebar_items()?;
    let localsend = crate::localsend::detect_localsend();
    let folder_customizations =
        crate::folder_customization::load_customizations_map().unwrap_or_default();

    Ok(StartupBundle {
        home,
        platform,
        sidebar_items,
        localsend_available: localsend.available,
        folder_customizations,
    })
}
