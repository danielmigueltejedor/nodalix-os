use std::process::Command;

pub fn label() -> String {
    Command::new("date")
        .arg("+%H:%M  %a %d")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "--:--".to_string())
}
