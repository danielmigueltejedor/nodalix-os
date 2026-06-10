use super::{command_exists, resolve_nodalix_bin, run_command_status, spawn_detached};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Shutdown,
    Reboot,
    Suspend,
    Hibernate,
    Lock,
    Logout,
}

#[derive(Debug, Clone)]
pub struct ActionAvailability {
    pub action: SystemAction,
    pub available: bool,
    pub reason: Option<String>,
}

pub fn availability() -> Vec<ActionAvailability> {
    [
        (SystemAction::Shutdown, can_systemctl(&["poweroff"])),
        (SystemAction::Reboot, can_systemctl(&["reboot"])),
        (SystemAction::Suspend, can_systemctl(&["suspend"])),
        (SystemAction::Hibernate, can_hibernate()),
        (SystemAction::Lock, can_lock()),
        (SystemAction::Logout, can_logout()),
    ]
    .into_iter()
    .map(|(action, (available, reason))| ActionAvailability {
        action,
        available,
        reason,
    })
    .collect()
}

pub fn suspend() -> Result<(), String> {
    ensure_available(SystemAction::Suspend)?;
    run_command_status("systemctl", &["suspend"]).map(|_| ())
}

pub fn reboot() -> Result<(), String> {
    ensure_available(SystemAction::Reboot)?;
    run_command_status("systemctl", &["reboot"]).map(|_| ())
}

pub fn poweroff() -> Result<(), String> {
    ensure_available(SystemAction::Shutdown)?;
    run_command_status("systemctl", &["poweroff"]).map(|_| ())
}

pub fn hibernate() -> Result<(), String> {
    ensure_available(SystemAction::Hibernate)?;
    run_command_status("systemctl", &["hibernate"]).map(|_| ())
}

pub fn lock_session() -> Result<(), String> {
    ensure_available(SystemAction::Lock)?;

    for name in ["nodalix-lock", "nodalix-system-lock"] {
        if let Some(bin) = resolve_nodalix_bin(name) {
            return spawn_detached(&bin.to_string_lossy(), &[]);
        }
    }

    if command_exists("loginctl") {
        return run_command_status("loginctl", &["lock-session"]).map(|_| ());
    }

    Err(
        "No se encontró nodalix-lock ni loginctl. Ejecuta: ./scripts/install-nodalix-lock.sh"
            .to_string(),
    )
}

pub fn logout_session() -> Result<(), String> {
    ensure_available(SystemAction::Logout)?;

    if command_exists("uwsm") {
        return run_command_status("uwsm", &["stop"]).map(|_| ());
    }
    if command_exists("hyprctl") {
        return run_command_status("hyprctl", &["dispatch", "exit"]).map(|_| ());
    }
    if command_exists("loginctl") {
        let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        return run_command_status("loginctl", &["terminate-user", &user]).map(|_| ());
    }

    Err("No se encontró uwsm, hyprctl ni loginctl para cerrar sesión".to_string())
}

fn ensure_available(action: SystemAction) -> Result<(), String> {
    let (available, reason) = match action {
        SystemAction::Shutdown => can_systemctl(&["poweroff"]),
        SystemAction::Reboot => can_systemctl(&["reboot"]),
        SystemAction::Suspend => can_systemctl(&["suspend"]),
        SystemAction::Hibernate => can_hibernate(),
        SystemAction::Lock => can_lock(),
        SystemAction::Logout => can_logout(),
    };

    if available {
        Ok(())
    } else {
        Err(reason.unwrap_or_else(|| "Acción no disponible en este sistema".to_string()))
    }
}

fn can_systemctl(_args: &[&str]) -> (bool, Option<String>) {
    if command_exists("systemctl") {
        (true, None)
    } else {
        (
            false,
            Some("systemctl no está instalado".to_string()),
        )
    }
}

fn can_hibernate() -> (bool, Option<String>) {
    if !command_exists("systemctl") {
        return (false, Some("systemctl no está instalado".to_string()));
    }
    let supports = std::fs::read_to_string("/sys/power/state")
        .map(|text| text.contains("disk"))
        .unwrap_or(false);
    if supports {
        (true, None)
    } else {
        (
            false,
            Some("Hibernación no soportada en este equipo".to_string()),
        )
    }
}

fn can_lock() -> (bool, Option<String>) {
    if resolve_nodalix_bin("nodalix-lock").is_some()
        || resolve_nodalix_bin("nodalix-system-lock").is_some()
    {
        return (true, None);
    }
    if command_exists("loginctl") {
        return (true, None);
    }
        (
            false,
            Some(
                "No se encontró nodalix-lock (ejecutable) ni loginctl. \
                 Ejecuta: ./scripts/install-nodalix-lock.sh"
                    .to_string(),
            ),
        )
}

fn can_logout() -> (bool, Option<String>) {
    if command_exists("uwsm") || command_exists("hyprctl") || command_exists("loginctl") {
        (true, None)
    } else {
        (
            false,
            Some("No hay herramienta de cierre de sesión disponible".to_string()),
        )
    }
}
