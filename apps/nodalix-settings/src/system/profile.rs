use nodalix_profile::{
    load_profile, save_profile, ShellPreferences, UserProfile, VisualPreferences,
};

pub fn load_user_profile() -> UserProfile {
    load_profile()
}

pub fn update_visual<F>(update: F) -> Result<std::path::PathBuf, String>
where
    F: FnOnce(&mut VisualPreferences),
{
    let mut profile = load_profile();
    update(&mut profile.visual);
    save_profile(&profile)
}

pub fn update_shell<F>(update: F) -> Result<std::path::PathBuf, String>
where
    F: FnOnce(&mut ShellPreferences),
{
    let mut profile = load_profile();
    update(&mut profile.shell);
    save_profile(&profile)
}

pub fn profile_store_paths() -> Vec<(String, String)> {
    use nodalix_profile::paths;
    vec![
        (
            "Perfil JSON".to_string(),
            paths::profile_path().display().to_string(),
        ),
        (
            "Avatar Nodalix".to_string(),
            paths::default_avatar_path().display().to_string(),
        ),
        (
            "Compatibilidad .face".to_string(),
            paths::home_dir().join(".face").display().to_string(),
        ),
    ]
}
