//! System identity for the About page.

use std::ffi::CStr;

/// Reads `KEY=value` lines, as in `/etc/os-release`. Values may be quoted.
pub fn parse_env_file(text: &str) -> std::collections::HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

/// The installed Nodalix version, from `/etc/nodalix-release`. Only a plain version is accepted:
/// anything else means the identity cannot be trusted.
pub fn nodalix_version(release: &str) -> Option<String> {
    let version = parse_env_file(release).remove("VERSION_ID")?;
    let core = version.split('-').next()?;
    let parts: Vec<&str> = core.split('.').collect();
    let plain = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    (plain && version != "0.0.0").then_some(version)
}

pub struct Info {
    pub hostname: String,
    pub os_name: String,
    pub nodalix_version: Option<String>,
    pub kernel: String,
    pub arch: String,
    pub cpu: Option<String>,
    pub memory: Option<u64>,
    pub disk: Option<u64>,
}

fn uname() -> (String, String) {
    // SAFETY: uname fills a plain struct that we own.
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut u) } != 0 {
        return (String::new(), String::new());
    }
    let text = |field: &[libc::c_char]| {
        // SAFETY: the kernel NUL-terminates every utsname field.
        unsafe { CStr::from_ptr(field.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    };
    (text(&u.release), text(&u.machine))
}

pub fn read() -> Info {
    let (kernel, arch) = uname();
    let os = std::fs::read_to_string("/etc/os-release")
        .map(|t| parse_env_file(&t))
        .unwrap_or_default();
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_string())
        .unwrap_or_default();
    Info {
        hostname,
        os_name: os
            .get("PRETTY_NAME")
            .or_else(|| os.get("NAME"))
            .cloned()
            .unwrap_or_else(|| "Nodalix".into()),
        nodalix_version: std::fs::read_to_string("/etc/nodalix-release")
            .ok()
            .and_then(|t| nodalix_version(&t)),
        kernel,
        arch,
        cpu: std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| monitor_core::cpu::parse_model(&t)),
        memory: monitor_core::mem::read().map(|m| m.total),
        disk: monitor_core::fs::usage("/").map(|(total, _)| total),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_plain_values_and_skips_comments() {
        let env = parse_env_file("# c\nNAME=\"Nodalix OS\"\nID=nodalix\nQ='x y'\n\nbroken\n");
        assert_eq!(env["NAME"], "Nodalix OS");
        assert_eq!(env["ID"], "nodalix");
        assert_eq!(env["Q"], "x y");
        assert!(!env.contains_key("broken"));
    }

    #[test]
    fn accepts_a_plain_version_and_a_pre_release() {
        assert_eq!(
            nodalix_version("VERSION_ID=\"0.6.0\"\n").as_deref(),
            Some("0.6.0")
        );
        assert_eq!(
            nodalix_version("VERSION_ID=0.6.0-beta.1\n").as_deref(),
            Some("0.6.0-beta.1")
        );
    }

    #[test]
    fn rejects_an_identity_that_cannot_be_trusted() {
        assert_eq!(nodalix_version("VERSION_ID=0.0.0\n"), None);
        assert_eq!(nodalix_version("VERSION_ID=rolling\n"), None);
        assert_eq!(nodalix_version("VERSION_ID=1.2\n"), None);
        assert_eq!(nodalix_version("NAME=x\n"), None);
    }

    #[test]
    fn reads_this_machine() {
        let info = read();
        assert!(!info.kernel.is_empty());
        assert!(info.memory.is_some_and(|m| m > 0));
        assert!(!info.os_name.is_empty());
    }
}
