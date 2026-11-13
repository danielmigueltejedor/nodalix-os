//! Cloud storage accounts, as the file manager and Settings share them. Settings sets an account
//! up with `rclone` and writes this list; the file manager reads it and starts the mount. A
//! mount is one `nodalix-cloud@<id>.service` user unit running `rclone mount`, so nothing here
//! runs a process: the module is the data, the file formats and the answers to `rclone`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Drive,
    OneDrive,
    Dropbox,
    #[serde(rename = "iclouddrive")]
    ICloudDrive,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::Drive,
        Provider::OneDrive,
        Provider::Dropbox,
        Provider::ICloudDrive,
    ];

    /// The name `rclone` gives the backend, and the one the list stores.
    pub fn key(self) -> &'static str {
        match self {
            Provider::Drive => "drive",
            Provider::OneDrive => "onedrive",
            Provider::Dropbox => "dropbox",
            Provider::ICloudDrive => "iclouddrive",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Provider::Drive => "Google Drive",
            Provider::OneDrive => "OneDrive",
            Provider::Dropbox => "Dropbox",
            Provider::ICloudDrive => "iCloud Drive",
        }
    }

    /// Whether signing in is a browser flow that hands `rclone` a token (all but iCloud, which
    /// takes the Apple ID and password).
    pub fn uses_browser(self) -> bool {
        self != Provider::ICloudDrive
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Letters, digits, `_` and `-`: it ends up in a unit name and in an `rclone` remote name.
    pub id: String,
    pub provider: Provider,
    pub name: String,
}

impl Account {
    pub fn remote(&self) -> String {
        remote_name(&self.id)
    }

    pub fn unit(&self) -> String {
        unit_name(&self.id)
    }

    /// The folder the account is mounted on, under the person's home.
    pub fn mount_dir(&self, home: &str) -> String {
        format!("{home}/{FOLDER}/{}", folder_name(self))
    }
}

/// Where mounts go, in the person's home.
pub const FOLDER: &str = "Nube";

pub fn remote_name(id: &str) -> String {
    format!("nodalix-{id}")
}

pub fn unit_name(id: &str) -> String {
    format!("nodalix-cloud@{id}.service")
}

/// The mount folder's name: the account's name with `/` swapped for `-`, or the provider's title
/// when the name is empty. The file manager works the folder out the same way.
pub fn folder_name(account: &Account) -> String {
    let name: String = account
        .name
        .trim()
        .chars()
        .map(|c| if c == '/' || c.is_control() { '-' } else { c })
        .collect();
    if name.is_empty() || name == "." || name == ".." {
        account.provider.title().to_owned()
    } else {
        name
    }
}

/// The accounts in the list's text. A damaged list, or an entry that is not understood, is
/// skipped rather than failing the whole page.
pub fn parse(text: &str) -> Vec<Account> {
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(text) else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Account>(v).ok())
        .filter(|a| is_valid_id(&a.id))
        .collect()
}

pub fn to_json(accounts: &[Account]) -> String {
    serde_json::to_string_pretty(accounts).unwrap_or_else(|_| "[]".to_owned())
}

pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// An id for a new account: the name in lowercase letters and digits, made unique.
pub fn new_id(provider: Provider, name: &str, taken: &[String]) -> String {
    let mut base: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    base = base.trim_matches('-').to_owned();
    if base.is_empty() {
        base = provider.key().to_owned();
    }
    base.truncate(32);
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// The user unit every account's mount is an instance of. The folder to mount on comes from a
/// file written for the instance, because a unit name carries only the id.
pub const UNIT_TEMPLATE: &str = "\
[Unit]
Description=Cloud storage %i (rclone)
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
EnvironmentFile=%h/.config/nodalix/cloud/%i.env
ExecStartPre=/usr/bin/mkdir -p ${NODALIX_CLOUD_DIR}
ExecStart=/usr/bin/rclone mount nodalix-%i: ${NODALIX_CLOUD_DIR} --vfs-cache-mode writes
ExecStop=/usr/bin/fusermount3 -u ${NODALIX_CLOUD_DIR}
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
";

/// The instance file that says where an account is mounted. systemd reads the value as it
/// stands up to the end of the line, quotes included, so the path is written bare.
pub fn instance_env(dir: &str) -> String {
    format!("NODALIX_CLOUD_DIR={}\n", dir.replace('\n', " "))
}

/// The token `rclone authorize` prints: a line holding one JSON object.
pub fn extract_token(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with('{') && l.ends_with('}') && l.contains("access_token"))
        .map(str::to_owned)
}

/// What `rclone config create … --non-interactive` says next: it is done, it failed, or it asks
/// a question to be answered with `rclone config update … --continue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Done,
    Failed(String),
    Ask {
        state: String,
        name: String,
        help: String,
        default: String,
    },
}

pub fn parse_step(output: &str) -> Step {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(output.trim()) else {
        // Nothing to parse is `rclone` having nothing more to ask.
        return Step::Done;
    };
    let text = |v: &serde_json::Value, key: &str| {
        v.get(key)
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let error = text(&value, "Error");
    if !error.is_empty() {
        return Step::Failed(error);
    }
    let state = text(&value, "State");
    if state.is_empty() {
        return Step::Done;
    }
    let option = value.get("Option").cloned().unwrap_or_default();
    let default = match option.get("Default") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    };
    Step::Ask {
        state,
        name: text(&option, "Name"),
        help: text(&option, "Help"),
        default,
    }
}

/// Whether a question is `rclone` asking for a two-factor code, which only the person has.
pub fn asks_for_code(name: &str) -> bool {
    let name = name.to_lowercase();
    name.contains("2fa") || name.contains("code") || name.contains("trust")
}

/// A verification code as Apple gives it: six digits.
pub fn is_valid_code(code: &str) -> bool {
    let code = code.trim();
    code.len() == 6 && code.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str, provider: Provider, name: &str) -> Account {
        Account {
            id: id.into(),
            provider,
            name: name.into(),
        }
    }

    #[test]
    fn the_list_uses_the_names_the_file_manager_reads() {
        let text = to_json(&[account("work", Provider::ICloudDrive, "Work")]);
        assert!(text.contains("\"provider\": \"iclouddrive\""));
        let back = parse(&text);
        assert_eq!(back, vec![account("work", Provider::ICloudDrive, "Work")]);
        let others = parse(
            r#"[{"id":"a","provider":"drive","name":"A"},{"id":"b","provider":"onedrive","name":"B"},{"id":"c","provider":"dropbox","name":"C"}]"#,
        );
        assert_eq!(others.len(), 3);
    }

    #[test]
    fn a_damaged_list_or_entry_is_skipped() {
        assert!(parse("not json").is_empty());
        assert!(parse("").is_empty());
        let mixed = parse(
            r#"[{"id":"a","provider":"drive","name":"A"},{"id":"b","provider":"mega","name":"B"},{"id":"../x","provider":"drive","name":"X"}]"#,
        );
        assert_eq!(mixed.len(), 1);
    }

    #[test]
    fn the_folder_is_the_name_with_slashes_swapped() {
        let a = account("x", Provider::Drive, "Work/Docs");
        assert_eq!(a.mount_dir("/home/u"), "/home/u/Nube/Work-Docs");
        let empty = account("y", Provider::Dropbox, "  ");
        assert_eq!(folder_name(&empty), "Dropbox");
        assert_eq!(
            folder_name(&account("z", Provider::Drive, "..")),
            "Google Drive"
        );
    }

    #[test]
    fn ids_are_valid_and_unique() {
        assert_eq!(new_id(Provider::Drive, "Mi Drive!", &[]), "mi-drive");
        assert_eq!(new_id(Provider::Dropbox, "***", &[]), "dropbox");
        let taken = vec!["work".to_owned(), "work-2".to_owned()];
        assert_eq!(new_id(Provider::Drive, "Work", &taken), "work-3");
        assert!(is_valid_id(&new_id(Provider::Drive, "Ünï ñ", &[])));
        assert!(!is_valid_id("a b"));
        assert!(!is_valid_id(""));
    }

    #[test]
    fn units_and_remotes_are_named_by_id() {
        let a = account("work", Provider::Drive, "Work");
        assert_eq!(a.unit(), "nodalix-cloud@work.service");
        assert_eq!(a.remote(), "nodalix-work");
        assert!(UNIT_TEMPLATE.contains("Type=notify"));
        assert!(UNIT_TEMPLATE.contains("nodalix-%i:"));
        assert_eq!(
            instance_env("/home/u/Nube/A B"),
            "NODALIX_CLOUD_DIR=/home/u/Nube/A B\n"
        );
    }

    #[test]
    fn the_token_is_the_json_line_of_the_authorize_output() {
        let out = "2026/11/12 Waiting for code…\nPaste the following into your remote machine --->\n{\"access_token\":\"abc\",\"expiry\":\"x\"}\n<---End paste\n";
        assert_eq!(
            extract_token(out).as_deref(),
            Some("{\"access_token\":\"abc\",\"expiry\":\"x\"}")
        );
        assert_eq!(extract_token("nothing here"), None);
    }

    #[test]
    fn rclone_steps_are_done_failed_or_a_question() {
        assert_eq!(parse_step(""), Step::Done);
        assert_eq!(
            parse_step(r#"{"State":"","Error":"","Result":""}"#),
            Step::Done
        );
        assert_eq!(
            parse_step(r#"{"State":"","Error":"bad token"}"#),
            Step::Failed("bad token".into())
        );
        let ask = parse_step(
            r#"{"State":"*oauth-islocal,teamdrive,,","Option":{"Name":"config_type","Help":"Type","Default":"onedrive"},"Error":""}"#,
        );
        assert_eq!(
            ask,
            Step::Ask {
                state: "*oauth-islocal,teamdrive,,".into(),
                name: "config_type".into(),
                help: "Type".into(),
                default: "onedrive".into()
            }
        );
    }

    #[test]
    fn a_code_is_six_digits() {
        assert!(is_valid_code("123456"));
        assert!(is_valid_code(" 123456 "));
        assert!(!is_valid_code("12345"));
        assert!(!is_valid_code("12a456"));
        assert!(!is_valid_code(""));
    }

    #[test]
    fn a_code_question_is_told_from_the_others() {
        assert!(asks_for_code("config_2fa"));
        assert!(asks_for_code("config_trust_token_code"));
        assert!(!asks_for_code("config_type"));
    }
}
