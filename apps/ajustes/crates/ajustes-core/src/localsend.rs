//! The state the session service reports for the LocalSend page (`GetLocalSend`).
//!
//! One JSON document, read as forgivingly as the update state: a missing field is empty and an
//! unknown one is ignored, so an older or newer service never breaks the page.

use serde::Deserialize;

/// The service accepts these keys in `SetLocalSendOption`; the values travel as JSON text.
pub const KEY_ALIAS: &str = "alias";
pub const KEY_DOWNLOAD_FOLDER: &str = "download-folder";
pub const KEY_AUTO_ACCEPT: &str = "auto-accept";
pub const KEY_START_ON_LOGIN: &str = "start-on-login";
pub const KEY_AUTO_DISABLE: &str = "auto-disable-enabled";
pub const KEY_AUTO_DISABLE_MINUTES: &str = "auto-disable-minutes";

/// Minutes the service accepts before LocalSend switches itself off.
pub const MINUTES: std::ops::RangeInclusive<u32> = 1..=1440;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub alias: String,
    #[serde(rename = "download-folder")]
    pub download_folder: String,
    #[serde(rename = "auto-accept")]
    pub auto_accept: bool,
    #[serde(rename = "start-on-login")]
    pub start_on_login: bool,
    #[serde(rename = "auto-disable-enabled")]
    pub auto_disable: bool,
    #[serde(rename = "auto-disable-minutes")]
    pub auto_disable_minutes: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Device {
    pub alias: String,
    #[serde(rename = "deviceModel")]
    pub model: Option<String>,
    pub fingerprint: String,
    pub favorite: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LocalSendState {
    /// The GLocalSend settings schema is there.
    pub installed: bool,
    /// The running LocalSend answers on the session bus.
    pub ready: bool,
    pub enabled: bool,
    pub message: String,
    pub config: Config,
    pub devices: Vec<Device>,
}

impl LocalSendState {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Looking for devices only makes sense while LocalSend runs and is switched on.
    pub fn can_search(&self) -> bool {
        self.ready && self.enabled
    }
}

/// A JSON string literal, which is how the service wants text values.
pub fn json_string(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = r#"{"installed":true,"ready":true,"enabled":true,
        "message":"Activo · 1 dispositivos cercanos",
        "config":{"alias":"Portátil","download-folder":"/home/a/Descargas","auto-accept":false,
                  "start-on-login":true,"auto-disable-enabled":true,"auto-disable-minutes":30},
        "favorites":["abc"],
        "devices":[{"alias":"Móvil","deviceModel":"Pixel","fingerprint":"abc","favorite":true},
                   {"alias":"PC","fingerprint":"def","favorite":false}]}"#;

    #[test]
    fn reads_what_the_service_sends() {
        let s = LocalSendState::parse(STATE).unwrap();
        assert!(s.installed && s.ready && s.enabled && s.can_search());
        assert_eq!(s.config.alias, "Portátil");
        assert_eq!(s.config.download_folder, "/home/a/Descargas");
        assert!(s.config.start_on_login && s.config.auto_disable && !s.config.auto_accept);
        assert_eq!(s.config.auto_disable_minutes, 30);
        assert_eq!(s.devices.len(), 2);
        assert_eq!(s.devices[0].model.as_deref(), Some("Pixel"));
        assert_eq!(s.devices[1].model, None);
        assert!(s.devices[0].favorite && !s.devices[1].favorite);
    }

    #[test]
    fn a_service_without_localsend_reports_only_a_message() {
        let s = LocalSendState::parse(
            r#"{"installed":false,"ready":false,"enabled":false,"devices":[],"config":{},
                "message":"GLocalSend no está instalado"}"#,
        )
        .unwrap();
        assert!(!s.installed && !s.can_search());
        assert_eq!(s.message, "GLocalSend no está instalado");
        assert!(s.devices.is_empty());
    }

    #[test]
    fn missing_and_unknown_fields_are_harmless() {
        let s = LocalSendState::parse(r#"{"something":"new"}"#).unwrap();
        assert_eq!(s, LocalSendState::default());
    }

    #[test]
    fn text_values_are_sent_as_json_strings() {
        assert_eq!(json_string("Mi \"equipo\""), r#""Mi \"equipo\"""#);
        assert_eq!(json_string("/home/a/Descargas"), r#""/home/a/Descargas""#);
    }
}
