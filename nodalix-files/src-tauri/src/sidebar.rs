use crate::files::{get_special_dirs_cached, SpecialDirs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub struct SidebarPin {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SidebarConfig {
    pub hidden_builtin: Vec<String>,
    pub pins: Vec<SidebarPin>,
    #[serde(default)]
    pub renamed: HashMap<String, String>,
}

#[derive(Serialize, Clone)]
pub struct SidebarItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: String,
    pub pinned: bool,
    pub custom: bool,
    pub can_rename: bool,
    pub can_remove: bool,
}

fn config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/nodalix-files/sidebar.json"))
}

pub fn load_config() -> Result<SidebarConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(SidebarConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub fn save_config(config: &SidebarConfig) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

static CONFIG_CACHE: Mutex<Option<SidebarConfig>> = Mutex::new(None);

pub fn config_cached() -> Result<SidebarConfig, String> {
    let mut cache = CONFIG_CACHE.lock().map_err(|_| "lock poisoned".to_string())?;
    if cache.is_none() {
        *cache = Some(load_config()?);
    }
    Ok(cache.clone().unwrap_or_default())
}

pub fn invalidate_sidebar_cache() {
    if let Ok(mut cache) = CONFIG_CACHE.lock() {
        *cache = None;
    }
}

fn builtin_items(dirs: &SpecialDirs, config: &SidebarConfig) -> Vec<SidebarItem> {
    let mut items = Vec::new();
    let builtins: Vec<(&str, &str, Option<&String>)> = vec![
        ("home", "Home", Some(&dirs.home)),
        ("desktop", "Desktop", dirs.desktop.as_ref()),
        ("downloads", "Downloads", dirs.downloads.as_ref()),
        ("documents", "Documents", dirs.documents.as_ref()),
        ("pictures", "Pictures", dirs.pictures.as_ref()),
        ("videos", "Videos", dirs.videos.as_ref()),
        ("music", "Music", dirs.music.as_ref()),
        ("data", "Data", dirs.data.as_ref()),
    ];

    for (id, label, path) in builtins {
        let Some(path) = path else { continue };
        if config.hidden_builtin.iter().any(|h| h == id) {
            continue;
        }
        items.push(SidebarItem {
            id: id.to_string(),
            label: label.to_string(),
            path: path.clone(),
            kind: id.to_string(),
            pinned: true,
            custom: false,
            can_rename: false,
            can_remove: false,
        });
    }
    items
}

pub fn build_sidebar_items() -> Result<Vec<SidebarItem>, String> {
    let dirs = get_special_dirs_cached()?;
    let config = config_cached()?;
    let mut items = builtin_items(&dirs, &config);

    for pin in &config.pins {
        if items.iter().any(|i| i.path == pin.path) {
            continue;
        }
        let label = config
            .renamed
            .get(&pin.id)
            .cloned()
            .unwrap_or_else(|| pin.label.clone());
        items.push(SidebarItem {
            id: pin.id.clone(),
            label,
            path: pin.path.clone(),
            kind: "custom".into(),
            pinned: true,
            custom: true,
            can_rename: true,
            can_remove: true,
        });
    }
    Ok(items)
}

#[tauri::command]
pub fn get_sidebar_items() -> Result<Vec<SidebarItem>, String> {
    crate::platform::debug_log("get_sidebar_items");
    build_sidebar_items()
}

#[tauri::command]
pub fn save_sidebar_items(config: SidebarConfig) -> Result<(), String> {
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn pin_sidebar_path(path: String, label: Option<String>) -> Result<(), String> {
    let mut config = config_cached()?;
    if config.pins.iter().any(|p| p.path == path) {
        return Ok(());
    }
    let id = format!("pin-{}", config.pins.len() + 1);
    let name = label.unwrap_or_else(|| {
        path.rsplit('/').next().unwrap_or("Folder").to_string()
    });
    config.pins.push(SidebarPin {
        id,
        label: name,
        path,
    });
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn unpin_sidebar_path(path: String) -> Result<(), String> {
    let mut config = config_cached()?;
    config.pins.retain(|p| p.path != path);
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn hide_builtin_sidebar(id: String) -> Result<(), String> {
    let mut config = config_cached()?;
    if !config.hidden_builtin.contains(&id) {
        config.hidden_builtin.push(id);
    }
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn show_builtin_sidebar(id: String) -> Result<(), String> {
    let mut config = config_cached()?;
    config.hidden_builtin.retain(|h| h != &id);
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn rename_sidebar_access(id: String, label: String) -> Result<(), String> {
    let mut config = config_cached()?;
    config.renamed.insert(id, label);
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}

#[tauri::command]
pub fn remove_sidebar_access(id: String) -> Result<(), String> {
    let mut config = config_cached()?;
    config.pins.retain(|p| p.id != id);
    config.renamed.remove(&id);
    save_config(&config)?;
    invalidate_sidebar_cache();
    Ok(())
}
