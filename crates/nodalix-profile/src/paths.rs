use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn nodalix_config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("nodalix")
}

pub fn user_config_dir_for_home(home: &Path) -> PathBuf {
    home.join(".config/nodalix/user")
}

pub fn profile_path_for_home(home: &Path) -> PathBuf {
    user_config_dir_for_home(home).join("profile.json")
}

pub fn default_avatar_path_for_home(home: &Path) -> PathBuf {
    user_config_dir_for_home(home).join("avatar.png")
}

pub fn profile_path() -> PathBuf {
    profile_path_for_home(&home_dir())
}

pub fn default_avatar_path() -> PathBuf {
    default_avatar_path_for_home(&home_dir())
}

pub fn user_config_dir() -> PathBuf {
    user_config_dir_for_home(&home_dir())
}
