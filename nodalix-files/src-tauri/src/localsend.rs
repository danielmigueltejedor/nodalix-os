use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct LocalSendInfo {
    pub available: bool,
    pub command: Option<String>,
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn flatpak_localsend() -> bool {
    Command::new("flatpak")
        .args(["info", "org.localsend.localsend"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn desktop_file_exists() -> bool {
    let dirs = [
        "/usr/share/applications",
        "/var/lib/flatpak/exports/share/applications",
        "/.local/share/applications",
    ];
    let home = std::env::var("HOME").unwrap_or_default();
    for dir in dirs {
        let path = if dir.starts_with('/') {
            Path::new(dir).join("localsend.desktop")
        } else {
            Path::new(&home).join(".local/share/applications/localsend.desktop")
        };
        if path.is_file() {
            return true;
        }
    }
    for name in ["localsend.desktop", "org.localsend.localsend.desktop"] {
        if Path::new("/usr/share/applications").join(name).is_file() {
            return true;
        }
    }
    false
}

pub fn detect_localsend() -> LocalSendInfo {
    if command_exists("localsend") {
        return LocalSendInfo {
            available: true,
            command: Some("localsend".into()),
        };
    }
    if flatpak_localsend() {
        return LocalSendInfo {
            available: true,
            command: Some("flatpak run org.localsend.localsend".into()),
        };
    }
    if desktop_file_exists() {
        return LocalSendInfo {
            available: true,
            command: Some("xdg-open".into()),
        };
    }
    LocalSendInfo {
        available: false,
        command: None,
    }
}

#[tauri::command]
pub fn get_localsend_info() -> LocalSendInfo {
    detect_localsend()
}

#[tauri::command]
pub fn send_with_localsend(paths: Vec<String>) -> Result<(), String> {
    let info = detect_localsend();
    if !info.available {
        return Err("LocalSend is not installed".into());
    }
    let cmd = info.command.unwrap_or_else(|| "localsend".into());
    if cmd.starts_with("flatpak") {
        let mut args: Vec<&str> = vec!["run", "org.localsend.localsend"];
        for p in &paths {
            args.push(p);
        }
        Command::new("flatpak")
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else if cmd == "xdg-open" {
        for p in paths {
            Command::new("xdg-open")
                .arg(p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    } else {
        let mut c = Command::new(&cmd);
        c.args(paths);
        c.spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}
