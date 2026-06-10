pub mod center;
pub mod workspaces;

use std::process::Command;

pub fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "command -v {} >/dev/null 2>&1",
            name.replace('\'', "'\\''")
        ))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn capture(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn spawn_detached(program: &str, args: &[&str]) {
    let _ = Command::new(program).args(args).spawn();
}
