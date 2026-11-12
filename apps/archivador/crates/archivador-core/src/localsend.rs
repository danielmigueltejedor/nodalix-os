//! Sharing with LocalSend: the devices the session service reports and the command that sends.
//!
//! The service is `com.nodalix.LocalSend` (`GetStatus` returns one JSON document). The sending
//! itself is `/usr/lib/nodalix/localsend-send`, run as a transient user unit so that it outlives
//! the window, exactly as the file-manager menu of the system has always done.

use std::path::PathBuf;

use serde::Deserialize;

pub const BUS_NAME: &str = "com.nodalix.LocalSend";
pub const OBJECT_PATH: &str = "/com/nodalix/LocalSend";
pub const INTERFACE: &str = "com.nodalix.LocalSend1";
pub const SEND_HELPER: &str = "/usr/lib/nodalix/localsend-send";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Device {
    pub alias: String,
    #[serde(rename = "deviceModel")]
    pub model: String,
    pub fingerprint: String,
    pub favorite: bool,
}

impl Device {
    /// The name in the list: `alias · model` unless the model adds nothing, `★` for favourites.
    pub fn label(&self) -> String {
        let alias = if self.alias.is_empty() {
            "LocalSend"
        } else {
            &self.alias
        };
        let name = if self.model.is_empty() || self.model.eq_ignore_ascii_case(alias) {
            alias.to_owned()
        } else {
            format!("{alias} · {}", self.model)
        };
        if self.favorite {
            format!("★ {name}")
        } else {
            name
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Status {
    enabled: bool,
    devices: Vec<Device>,
}

/// The nearby devices in `GetStatus`'s answer. Nothing is listed while LocalSend is switched off;
/// text that is not the expected JSON is an error.
pub fn devices(status_json: &str) -> Result<Vec<Device>, serde_json::Error> {
    let status: Status = serde_json::from_str(status_json)?;
    Ok(if status.enabled {
        status.devices
    } else {
        Vec::new()
    })
}

/// The command that sends `paths` to a device. `unit_id` makes the transient unit's name unique.
pub fn send_command(device: &Device, paths: &[PathBuf], unit_id: &str) -> Vec<String> {
    let mut command: Vec<String> = [
        "systemd-run",
        "--user",
        "--collect",
        "--quiet",
        &format!("--unit=nodalix-localsend-send-{unit_id}"),
        "--",
        SEND_HELPER,
        "--fingerprint",
        &device.fingerprint,
        "--alias",
        &device.alias,
        "--",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    command.extend(paths.iter().map(|p| p.to_string_lossy().into_owned()));
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"{"enabled":true,"devices":[
        {"fingerprint":"fp1","alias":"Pixel","deviceModel":"Pixel 9","favorite":true},
        {"fingerprint":"fp2","alias":"Mac","deviceModel":"mac"},
        {"fingerprint":"fp3","alias":"","deviceModel":""}],"other":1}"#;

    #[test]
    fn devices_are_read_and_named_like_the_system_menu_names_them() {
        let list = devices(STATUS).unwrap();
        let labels: Vec<_> = list.iter().map(Device::label).collect();
        assert_eq!(labels, ["★ Pixel · Pixel 9", "Mac", "LocalSend"]);
        assert_eq!(list[1].fingerprint, "fp2");
    }

    #[test]
    fn nothing_is_listed_while_localsend_is_off_or_the_answer_is_garbage() {
        assert!(devices(r#"{"enabled":false,"devices":[{"alias":"x"}]}"#)
            .unwrap()
            .is_empty());
        assert!(devices("{}").unwrap().is_empty());
        assert!(devices("not json").is_err());
    }

    #[test]
    fn the_send_command_runs_the_helper_in_a_transient_unit() {
        let device = Device {
            alias: "Pixel".into(),
            fingerprint: "fp1".into(),
            ..Device::default()
        };
        let command = send_command(
            &device,
            &[PathBuf::from("/a/b c.txt"), PathBuf::from("/d")],
            "abc123",
        );
        assert_eq!(
            command,
            [
                "systemd-run",
                "--user",
                "--collect",
                "--quiet",
                "--unit=nodalix-localsend-send-abc123",
                "--",
                "/usr/lib/nodalix/localsend-send",
                "--fingerprint",
                "fp1",
                "--alias",
                "Pixel",
                "--",
                "/a/b c.txt",
                "/d",
            ]
        );
    }
}
