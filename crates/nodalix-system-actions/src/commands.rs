use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn run_command_status(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|e| format!("No se pudo ejecutar {command}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{command} terminó con código {:?}. Revisa permisos de logind/systemd.",
            status.code()
        ))
    }
}

pub fn spawn_detached(command: &str, args: &[&str]) -> Result<(), String> {
    let path = Path::new(command);
    if path.is_absolute() && !is_executable(path) {
        return Err(format!(
            "Permiso denegado o no ejecutable: {command}. Ejecuta: chmod +x {command}"
        ));
    }

    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!(
                    "Permiso denegado al ejecutar {command}. Ejecuta: chmod +x {command}"
                )
            } else {
                format!("No se pudo iniciar {command}: {e}")
            }
        })
}

pub fn command_exists(command: &str) -> bool {
    resolve_executable(command).is_some()
}

pub fn resolve_nodalix_bin(name: &str) -> Option<PathBuf> {
    if let Some(path) = resolve_executable(name) {
        return Some(path);
    }

    let home = std::env::var_os("HOME")?;
    for base in ["Projects/nodalix-os", "Proyectos/nodalix-os"] {
        let candidate = PathBuf::from(&home).join(base).join("local/bin").join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    let system = PathBuf::from("/usr/local/bin").join(name);
    if is_executable(&system) {
        return Some(system);
    }

    None
}

fn resolve_executable(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name);
    if path.is_absolute() {
        return is_executable(&path).then_some(path);
    }

    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            is_executable(&candidate).then_some(candidate)
        })
    })
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
        && fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}
