use crate::paths::{default_avatar_path_for_home, profile_path_for_home, user_config_dir_for_home};
use crate::visual::VisualPreferences;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ShellPreferences {
    /// `waybar` or `nodalix-bar`.
    pub active_bar: String,
    pub control_center_compact: bool,
}

impl Default for ShellPreferences {
    fn default() -> Self {
        Self {
            active_bar: "waybar".to_string(),
            control_center_compact: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UserProfile {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Explicit avatar path; defaults to `~/.config/nodalix/user/avatar.png` when set via Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    pub visual: VisualPreferences,
    pub shell: ShellPreferences,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            username: std::env::var("USER").unwrap_or_else(|_| "usuario".to_string()),
            display_name: None,
            avatar_path: None,
            visual: VisualPreferences::default(),
            shell: ShellPreferences::default(),
        }
    }
}

pub fn load_profile() -> UserProfile {
    load_profile_for_home(&crate::paths::home_dir())
}

pub fn load_profile_for_home(home: &Path) -> UserProfile {
    let path = profile_path_for_home(home);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(mut profile) = serde_json::from_str::<UserProfile>(&raw) {
            if profile.username.is_empty() {
                profile.username = home
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("usuario")
                    .to_string();
            }
            return profile;
        }
    }
    UserProfile {
        username: home
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("usuario")
            .to_string(),
        ..UserProfile::default()
    }
}

pub fn save_profile(profile: &UserProfile) -> Result<PathBuf, String> {
    save_profile_for_home(&crate::paths::home_dir(), profile)
}

pub fn save_profile_for_home(home: &Path, profile: &UserProfile) -> Result<PathBuf, String> {
    let dir = user_config_dir_for_home(home);
    fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear {}: {e}", dir.display()))?;
    let path = profile_path_for_home(home);
    let raw = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("No se pudo serializar el perfil: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("No se pudo escribir {}: {e}", path.display()))?;
    Ok(path)
}

/// Copies the chosen image into the Nodalix profile store and keeps `.face` in sync.
pub fn set_avatar_from_file(source: &Path) -> Result<PathBuf, String> {
    set_avatar_from_file_for_home(&crate::paths::home_dir(), source)
}

pub fn set_avatar_from_file_for_home(home: &Path, source: &Path) -> Result<PathBuf, String> {
    if !source.is_file() {
        return Err(format!("No existe la imagen: {}", source.display()));
    }

    let dir = user_config_dir_for_home(home);
    fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear {}: {e}", dir.display()))?;

    let dest = default_avatar_path_for_home(home);
    fs::copy(source, &dest).map_err(|e| format!("No se pudo copiar la imagen: {e}"))?;

    let face = home.join(".face");
    let _ = fs::copy(source, &face);

    let mut profile = load_profile_for_home(home);
    profile.avatar_path = Some(dest.display().to_string());
    save_profile_for_home(home, &profile)?;

    Ok(dest)
}

pub fn configured_avatar_path(home: &Path, profile: &UserProfile) -> Option<PathBuf> {
    if let Some(path) = profile.avatar_path.as_ref() {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let default = default_avatar_path_for_home(home);
    if default.is_file() {
        return Some(default);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_roundtrip() {
        let profile = UserProfile::default();
        let raw = serde_json::to_string(&profile).unwrap();
        let parsed: UserProfile = serde_json::from_str(&raw).unwrap();
        assert_eq!(profile.visual.color_scheme, parsed.visual.color_scheme);
        assert_eq!(parsed.shell.active_bar, "waybar");
    }
}
