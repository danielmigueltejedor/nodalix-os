use super::{resolve_nodalix_bin, run_command_status};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeFile {
    pub name: Option<String>,
    pub hex: Option<String>,
}

pub const ACCENT_PRESETS: &[(&str, &str, &str)] = &[
    ("purple", "Morado", "#cba6f7"),
    ("blue", "Azul", "#89b4fa"),
    ("pink", "Rosa", "#f5c2e7"),
    ("green", "Verde", "#a6e3a1"),
    ("orange", "Naranja", "#fab387"),
];

pub fn theme_path() -> PathBuf {
    dirs_home().join(".config/nodalix/theme.json")
}

pub fn load_current() -> ThemeFile {
    let path = theme_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(theme) = serde_json::from_str::<ThemeFile>(&raw) {
            return theme;
        }
    }
    ThemeFile {
        name: Some("Nodalix Purple".to_string()),
        hex: Some("#cba6f7".to_string()),
    }
}

pub fn apply_accent_preset(preset: &str) -> Result<(), String> {
    let bin = resolve_nodalix_bin("nodalix-accent-color")
        .ok_or_else(|| "nodalix-accent-color no encontrado en PATH".to_string())?;
    run_command_status(&bin.to_string_lossy(), &[preset]).map(|_| ())
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}
