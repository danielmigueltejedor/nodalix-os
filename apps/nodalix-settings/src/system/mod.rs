pub mod audio;
pub mod bluetooth;
pub mod displays;
pub mod info;
pub mod network;
pub mod power;
pub mod storage;
pub mod theme;
pub mod updates;
pub mod users;
pub mod wifi;

use std::{path::PathBuf, process::{Command, Stdio}};

pub fn run_read_only(command: &str, args: &[&str]) -> Option<String> {
    run_command_stdout(command, args).ok().filter(|text| !text.is_empty())
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
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|e| format!("No se pudo ejecutar {command}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} terminó con código {:?}", status.code()))
    }
}

pub fn spawn_detached(command: &str, args: &[&str]) -> Result<(), String> {
    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No se pudo iniciar {command}: {e}"))
}

pub fn command_exists(command: &str) -> bool {
    if PathBuf::from(command).is_absolute() {
        return PathBuf::from(command).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(command);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

pub fn resolve_nodalix_bin(name: &str) -> Option<PathBuf> {
    if command_exists(name) {
        return which_in_path(name);
    }
    let home = std::env::var_os("HOME")?;
    for base in ["Projects/nodalix-os", "Proyectos/nodalix-os"] {
        let candidate = PathBuf::from(&home).join(base).join("local/bin").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
  let system = PathBuf::from("/usr/local/bin").join(name);
    if system.is_file() {
        return Some(system);
    }
    None
}

fn which_in_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}
