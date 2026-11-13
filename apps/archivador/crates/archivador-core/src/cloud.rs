//! The cloud accounts the user set up in Settings, which show up in the sidebar.
//!
//! Settings keeps them in `~/.config/nodalix/cloud.json`, a list of `{id, provider, name}`; each
//! one is an rclone remote called `nodalix-<id>`, mounted by the user service
//! `nodalix-cloud@<id>.service` at `~/Nube/<name>`. Archivador only reads that list, and starts
//! and stops the service. It never writes the configuration.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The folder, in the home folder, that holds the mounts.
pub const FOLDER: &str = "Nube";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Drive,
    Onedrive,
    Dropbox,
    Iclouddrive,
}

impl Provider {
    pub fn title(self) -> &'static str {
        match self {
            Provider::Drive => "Google Drive",
            Provider::Onedrive => "OneDrive",
            Provider::Dropbox => "Dropbox",
            Provider::Iclouddrive => "iCloud Drive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub provider: Provider,
    pub name: String,
}

#[derive(Deserialize)]
struct Raw {
    id: String,
    provider: Provider,
    #[serde(default)]
    name: String,
}

/// An id is part of a service name and of a command, so it is plain or nothing.
fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A name that is safe as a folder: no slashes, and not `.` or `..`.
fn folder_name(name: &str, provider: Provider) -> String {
    let clean: String = name
        .trim()
        .chars()
        .map(|c| if c == '/' || c.is_control() { '-' } else { c })
        .collect();
    if clean.is_empty() || clean == "." || clean == ".." {
        provider.title().to_owned()
    } else {
        clean
    }
}

/// The accounts in `text`, the JSON Settings writes. An entry that does not make sense is left
/// out, and anything unreadable gives no accounts: a broken file must not keep the app from
/// opening.
pub fn parse(text: &str) -> Vec<Account> {
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(text) else {
        return Vec::new();
    };
    let mut accounts: Vec<Account> = Vec::new();
    for value in values {
        let Ok(raw) = serde_json::from_value::<Raw>(value) else {
            continue;
        };
        if !plain_id(&raw.id) || accounts.iter().any(|a| a.id == raw.id) {
            continue;
        }
        let name = folder_name(&raw.name, raw.provider);
        accounts.push(Account {
            id: raw.id,
            provider: raw.provider,
            name,
        });
    }
    accounts
}

impl Account {
    /// Where the account is mounted.
    pub fn mount_point(&self, home: &Path) -> PathBuf {
        home.join(FOLDER).join(&self.name)
    }

    /// The user service that mounts it.
    pub fn unit(&self) -> String {
        format!("nodalix-cloud@{}.service", self.id)
    }
}

/// Whether `path` is a mount point, going by the text of `/proc/self/mountinfo`.
pub fn is_mounted(mountinfo: &str, path: &Path) -> bool {
    mountinfo
        .lines()
        .filter_map(|line| line.split(' ').nth(4))
        .any(|point| unescape(point) == path.as_os_str().to_string_lossy())
}

/// The kernel writes a space, a tab, a newline and a backslash in a path as `\040`, `\011`,
/// `\012` and `\134`.
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = (bytes[i] == b'\\' && i + 3 < bytes.len())
            .then(|| std::str::from_utf8(&bytes[i + 1..i + 4]).ok())
            .flatten()
            .and_then(|digits| u8::from_str_radix(digits, 8).ok());
        match octal {
            Some(byte) => {
                out.push(byte);
                i += 4;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_accounts_settings_writes() {
        let accounts = parse(
            r#"[{"id":"a1","provider":"drive","name":"Work"},
                {"id":"b2","provider":"iclouddrive","name":""}]"#,
        );
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].provider, Provider::Drive);
        assert_eq!(accounts[0].name, "Work");
        assert_eq!(accounts[1].name, "iCloud Drive");
        assert_eq!(accounts[0].unit(), "nodalix-cloud@a1.service");
        assert_eq!(
            accounts[0].mount_point(Path::new("/home/u")),
            Path::new("/home/u/Nube/Work")
        );
    }

    #[test]
    fn leaves_out_what_does_not_make_sense() {
        let text = r#"[
            {"id":"ok","provider":"dropbox","name":"A"},
            {"id":"ok","provider":"dropbox","name":"Again"},
            {"id":"a b","provider":"dropbox","name":"Spaces"},
            {"id":"x@y","provider":"dropbox","name":"At"},
            {"id":"","provider":"dropbox","name":"Empty"},
            {"id":"p","provider":"mega","name":"Unknown"},
            {"provider":"drive"},
            7
        ]"#;
        let accounts = parse(text);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "A");
        assert!(parse("not json").is_empty());
        assert!(parse("{}").is_empty());
    }

    #[test]
    fn a_name_cannot_leave_the_mount_folder() {
        let accounts = parse(
            r#"[{"id":"a","provider":"drive","name":"../../etc"},
                {"id":"b","provider":"onedrive","name":".."}]"#,
        );
        for account in &accounts {
            let point = account.mount_point(Path::new("/home/u"));
            assert_eq!(point.parent(), Some(Path::new("/home/u/Nube")));
            assert_ne!(point.file_name().unwrap(), "..");
        }
    }

    #[test]
    fn finds_mount_points() {
        let info = "36 35 98:0 / /home/u/Nube/My\\040Drive rw,nosuid - fuse.rclone remote rw\n\
                    25 1 8:1 / / rw - ext4 /dev/sda1 rw\n";
        assert!(is_mounted(info, Path::new("/home/u/Nube/My Drive")));
        assert!(is_mounted(info, Path::new("/")));
        assert!(!is_mounted(info, Path::new("/home/u/Nube")));
        assert!(!is_mounted("", Path::new("/")));
    }
}
