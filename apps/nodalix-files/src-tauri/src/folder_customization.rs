use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct FolderStyle {
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Store {
    folders: HashMap<String, FolderStyle>,
}

fn config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/nodalix-files/folder-customization.json"))
}

fn load_store() -> Result<Store, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Store::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_store(store: &Store) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

static CACHE: Mutex<Option<Store>> = Mutex::new(None);

fn store_cached() -> Result<Store, String> {
    let mut cache = CACHE.lock().map_err(|_| "lock poisoned".to_string())?;
    if cache.is_none() {
        *cache = Some(load_store()?);
    }
    Ok(cache.clone().unwrap_or_default())
}

fn invalidate_cache() {
    if let Ok(mut cache) = CACHE.lock() {
        *cache = None;
    }
}

pub fn load_customizations_map() -> Result<HashMap<String, FolderStyle>, String> {
    Ok(store_cached()?.folders)
}

#[tauri::command]
pub fn get_folder_customizations() -> Result<HashMap<String, FolderStyle>, String> {
    load_customizations_map()
}

#[tauri::command]
pub fn set_folder_color(path: String, color: Option<String>) -> Result<(), String> {
    let mut store = store_cached()?;
    let entry = store.folders.entry(path).or_default();
    entry.color = color;
    save_store(&store)?;
    invalidate_cache();
    Ok(())
}

#[tauri::command]
pub fn set_folder_icon(path: String, icon: Option<String>) -> Result<(), String> {
    let mut store = store_cached()?;
    let entry = store.folders.entry(path).or_default();
    entry.icon = icon;
    save_store(&store)?;
    invalidate_cache();
    Ok(())
}
