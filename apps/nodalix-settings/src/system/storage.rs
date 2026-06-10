use super::run_command_stdout;

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub filesystem: String,
    pub mount: String,
    pub size: String,
    pub used: String,
    pub avail: String,
    pub use_percent: u8,
}

pub fn list_mounts() -> Result<Vec<MountPoint>, String> {
    let out = run_command_stdout(
        "df",
        &[
            "-h", "-T", "-x", "tmpfs", "-x", "devtmpfs", "-x", "squashfs",
        ],
    )?;
    let mut mounts = Vec::new();
    for (i, line) in out.lines().enumerate() {
        if i == 0 {
            continue;
        }
        let mut parts = line.split_whitespace();
        let filesystem = parts.next().unwrap_or("").to_string();
        let fstype = parts.next().unwrap_or("");
        let size = parts.next().unwrap_or("").to_string();
        let used = parts.next().unwrap_or("").to_string();
        let avail = parts.next().unwrap_or("").to_string();
        let pct_raw = parts.next().unwrap_or("0%");
        let mount = parts.next().unwrap_or("").to_string();
        let use_percent = pct_raw.trim_end_matches('%').parse().unwrap_or(0);
        if mount.is_empty() {
            continue;
        }
        let _ = fstype;
        mounts.push(MountPoint {
            filesystem,
            mount,
            size,
            used,
            avail,
            use_percent,
        });
    }
    Ok(mounts)
}

pub fn open_file_manager(path: &str) -> Result<(), String> {
    if super::command_exists("xdg-open") {
        return super::spawn_detached("xdg-open", &[path]);
    }
    Err("xdg-open no disponible".to_string())
}
