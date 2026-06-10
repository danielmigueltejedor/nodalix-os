pub mod audio;
pub mod bluetooth;
pub mod displays;
pub mod info;
pub mod network;
pub mod power;
pub mod profile;
pub mod storage;
pub mod theme;
pub mod updates;
pub mod users;
pub mod wifi;

use std::process::Command;

pub use nodalix_system_actions::{command_exists, resolve_nodalix_bin, spawn_detached};

pub fn run_read_only(command: &str, args: &[&str]) -> Option<String> {
    run_command_stdout(command, args)
        .ok()
        .filter(|text| !text.is_empty())
}

pub fn run_command_stdout(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| format!("No se pudo ejecutar {command}: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{command} falló")
        } else {
            stderr
        });
    }
    Ok(stdout)
}

pub fn run_command_status(command: &str, args: &[&str]) -> Result<(), String> {
    nodalix_system_actions::run_command_status(command, args)
}
