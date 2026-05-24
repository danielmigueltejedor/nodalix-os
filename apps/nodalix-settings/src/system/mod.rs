pub mod audio;
pub mod displays;
pub mod network;
pub mod power;
pub mod storage;
pub mod updates;
pub mod users;

use std::{process::Command, time::Duration};

pub fn run_read_only(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn unavailable() -> String {
    "No disponible".to_string()
}

pub fn pending_label(value: Option<String>) -> String {
    value.unwrap_or_else(unavailable)
}

pub fn _probe_timeout() -> Duration {
    Duration::from_secs(2)
}
