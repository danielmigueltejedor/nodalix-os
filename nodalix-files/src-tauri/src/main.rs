#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Serialize, Clone)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: Option<u64>,
}

#[derive(Serialize)]
struct SpecialDirs {
    home: String,
    desktop: Option<String>,
    downloads: Option<String>,
    documents: Option<String>,
    pictures: Option<String>,
    videos: Option<String>,
    music: Option<String>,
    data: Option<String>,
}

fn error(msg: impl Into<String>) -> String {
    msg.into()
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_string())
}

fn expand_user_dir(value: &str, home: &Path) -> PathBuf {
    let expanded = value.replace("$HOME", &home.to_string_lossy());
    PathBuf::from(expanded)
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
            return path_to_string(&path);
        }
    }

    if let Some(path) = read_user_dirs_entry(home, user_dirs_key) {
        if path.is_dir() {
            return path_to_string(&path);
        }
    }

    let fallback = home.join(fallback_name);
    if fallback.is_dir() {
        return path_to_string(&fallback);
    }

    None
}

fn path_to_string(path: &Path) -> Option<String> {
    fs::canonicalize(path)
        .ok()
        .or_else(|| Some(path.to_path_buf()))
        .map(|p| p.to_string_lossy().into_owned())
}

fn modified_secs(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

#[tauri::command]
fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(error(format!("Not a directory: {path}")));
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let read_dir = fs::read_dir(&dir).map_err(|e| format!("Cannot read {path}: {e}"))?;

    for item in read_dir {
        let item = item.map_err(|e| e.to_string())?;
        let file_name = item.file_name().to_string_lossy().into_owned();
        if file_name == "." || file_name == ".." {
            continue;
        }

        let file_path = item.path();
        let meta = item.metadata().map_err(|e| e.to_string())?;
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let full_path = path_to_string(&file_path).unwrap_or_else(|| file_path.to_string_lossy().into_owned());

        entries.push(FileEntry {
            name: file_name,
            path: full_path,
            is_dir,
            size,
            modified: modified_secs(&file_path),
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
fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(error(format!("Path does not exist: {path}")));
    }

    let status = Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to launch xdg-open: {e}"))?;

    drop(status);
    Ok(())
}

#[tauri::command]
fn create_folder(parent: String, name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(error("Folder name cannot be empty"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(error("Folder name cannot contain path separators"));
    }

    let parent_path = PathBuf::from(&parent);
    if !parent_path.is_dir() {
        return Err(error(format!("Parent is not a directory: {parent}")));
    }

    let new_path = parent_path.join(trimmed);
    if new_path.exists() {
        return Err(error(format!("Already exists: {}", new_path.display())));
    }

    fs::create_dir(&new_path).map_err(|e| e.to_string())?;
    path_to_string(&new_path).ok_or_else(|| "Created but path unavailable".to_string())
}

#[tauri::command]
fn rename_path(old_path: String, new_name: String) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(error("New name cannot be empty"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(error("New name cannot contain path separators"));
    }

    let source = PathBuf::from(&old_path);
    if !source.exists() {
        return Err(error(format!("Path does not exist: {old_path}")));
    }

    let parent = source
        .parent()
        .ok_or_else(|| error("Cannot resolve parent directory"))?;
    let dest = parent.join(trimmed);

    if dest.exists() {
        return Err(error(format!("Already exists: {}", dest.display())));
    }

    fs::rename(&source, &dest).map_err(|e| e.to_string())?;
    path_to_string(&dest).ok_or_else(|| "Renamed but path unavailable".to_string())
}

#[tauri::command]
fn trash_path(path: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(error(format!("Path does not exist: {path}")));
    }

    match Command::new("gio").args(["trash", &path]).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(error(format!("gio trash failed (exit {status})"))),
        Err(e) => Err(error(format!(
            "gio trash unavailable ({e}). Install glib2 (gio) to use trash."
        ))),
    }
}

#[tauri::command]
fn get_special_dirs() -> Result<SpecialDirs, String> {
    let home = home_dir()?;
    let home_str = path_to_string(&home).ok_or_else(|| "Cannot resolve HOME path".to_string())?;

    let data = if Path::new("/data").is_dir() {
        Some("/data".to_string())
    } else {
        None
    };

    Ok(SpecialDirs {
        home: home_str,
        desktop: resolve_user_dir(&home, "XDG_DESKTOP_DIR", "XDG_DESKTOP_DIR", "Desktop"),
        downloads: resolve_user_dir(&home, "XDG_DOWNLOAD_DIR", "XDG_DOWNLOAD_DIR", "Downloads"),
        documents: resolve_user_dir(&home, "XDG_DOCUMENTS_DIR", "XDG_DOCUMENTS_DIR", "Documents"),
        pictures: resolve_user_dir(&home, "XDG_PICTURES_DIR", "XDG_PICTURES_DIR", "Pictures"),
        videos: resolve_user_dir(&home, "XDG_VIDEOS_DIR", "XDG_VIDEOS_DIR", "Videos"),
        music: resolve_user_dir(&home, "XDG_MUSIC_DIR", "XDG_MUSIC_DIR", "Music"),
        data,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_directory,
            open_path,
            create_folder,
            rename_path,
            trash_path,
            get_special_dirs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
