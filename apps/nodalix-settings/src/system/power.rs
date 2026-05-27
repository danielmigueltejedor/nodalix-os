use super::{run_command_status, run_read_only, resolve_nodalix_bin, spawn_detached};

pub const PROFILES: [(&str, &str); 3] = [
    ("performance", "Rendimiento"),
    ("balanced", "Equilibrado"),
    ("power-saver", "Ahorro"),
];

pub fn active_profile() -> String {
    run_read_only("powerprofilesctl", &["get"]).unwrap_or_else(|| "balanced".to_string())
}

pub fn set_profile(profile: &str) -> Result<(), String> {
    if !PROFILES.iter().any(|(id, _)| *id == profile) {
        return Err(format!("Perfil no válido: {profile}"));
    }
    run_command_status("powerprofilesctl", &["set", profile]).map(|_| ())
}

pub fn suspend() -> Result<(), String> {
    run_command_status("systemctl", &["suspend"]).map(|_| ())
}

pub fn reboot() -> Result<(), String> {
    run_command_status("systemctl", &["reboot"]).map(|_| ())
}

pub fn poweroff() -> Result<(), String> {
    run_command_status("systemctl", &["poweroff"]).map(|_| ())
}

pub fn lock_session() -> Result<(), String> {
    if let Some(bin) = resolve_nodalix_bin("nodalix-system-lock") {
        return spawn_detached(&bin.to_string_lossy(), &[]);
    }
    run_command_status("hyprlock", &[]).map(|_| ())
}

pub fn logout_session() -> Result<(), String> {
    if super::command_exists("uwsm") {
        return run_command_status("uwsm", &["stop"]).map(|_| ());
    }
    if super::command_exists("hyprctl") {
        return run_command_status("hyprctl", &["dispatch", "exit"]).map(|_| ());
    }
    if super::command_exists("loginctl") {
        let user = std::env::var("USER").unwrap_or_else(|_| "dani".to_string());
        return run_command_status("loginctl", &["terminate-user", &user]).map(|_| ());
    }
    Err("No se encontró uwsm, hyprctl ni loginctl para cerrar sesión".to_string())
}
