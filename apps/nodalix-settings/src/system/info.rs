use super::run_read_only;
use std::path::PathBuf;

pub fn hostname() -> String {
    run_read_only("hostname", &[]).unwrap_or_else(|| "—".to_string())
}

pub fn nodalix_version() -> String {
    if let Ok(v) = std::fs::read_to_string(version_file()) {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(out) = run_read_only("nodalix-version", &[]) {
        return out.lines().next().unwrap_or("dev").to_string();
    }
    "dev".to_string()
}

fn version_file() -> PathBuf {
    for candidate in [
        PathBuf::from("/etc/nodalix/VERSION"),
        home_path("Projects/nodalix-os/VERSION"),
        home_path("Proyectos/nodalix-os/VERSION"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("/etc/nodalix/VERSION")
}

fn home_path(rel: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(rel))
        .unwrap_or_else(|| PathBuf::from(rel))
}
