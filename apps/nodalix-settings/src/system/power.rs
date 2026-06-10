use super::run_read_only;
use nodalix_system_actions::{
    hibernate as sys_hibernate, lock_session as sys_lock, logout_session as sys_logout,
    poweroff as sys_poweroff, reboot as sys_reboot, suspend as sys_suspend,
};

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
    super::run_command_status("powerprofilesctl", &["set", profile]).map(|_| ())
}

pub fn suspend() -> Result<(), String> {
    sys_suspend()
}

pub fn reboot() -> Result<(), String> {
    sys_reboot()
}

pub fn poweroff() -> Result<(), String> {
    sys_poweroff()
}

pub fn lock_session() -> Result<(), String> {
    sys_lock()
}

pub fn logout_session() -> Result<(), String> {
    sys_logout()
}

#[allow(dead_code)]
pub fn hibernate() -> Result<(), String> {
    sys_hibernate()
}

pub fn action_available(action: nodalix_system_actions::SystemAction) -> (bool, Option<String>) {
    nodalix_system_actions::availability()
        .into_iter()
        .find(|entry| entry.action == action)
        .map(|entry| (entry.available, entry.reason))
        .unwrap_or((false, Some("Acción desconocida".to_string())))
}
