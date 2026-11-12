//! The state the session service (`com.nodalix.Settings`) reports for the Updates page.
//!
//! The service sends one JSON document. Every field has a default so that an older or newer
//! service never breaks the page: a missing field is empty, an unknown one is ignored.

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Source {
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub available: bool,
    pub count: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UpdateState {
    pub busy: bool,
    pub checking: bool,
    pub message: String,
    pub log: String,
    pub checked_at: String,
    pub identity: String,
    pub reboot_required: bool,
    pub reboot_mandatory: bool,
    pub reboot_reason: String,
    pub phase: String,
    pub sources: Vec<Source>,
}

impl UpdateState {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The page can start a check or an update only when nothing else is running.
    pub fn idle(&self) -> bool {
        !self.busy && !self.checking
    }

    /// Sources that exist on this system.
    pub fn available_sources(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter().filter(|s| s.available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_the_service_sends() {
        let state = UpdateState::parse(
            r#"{"busy":false,"checking":false,"message":"Nodalix está actualizado","log":"","checked_at":"12:30",
                "reboot_required":false,"identity":"Nodalix 0.5.2 · stable","phase":"idle",
                "sources":[{"kind":"nodalix","title":"Nodalix","detail":"Al día","available":true},
                           {"kind":"flatpak","title":"Aplicaciones Flatpak","detail":"No instalado","available":false}],
                "automatic":{"nodalix":{"enabled":true}}}"#,
        )
        .unwrap();
        assert_eq!(state.message, "Nodalix está actualizado");
        assert_eq!(state.sources.len(), 2);
        assert_eq!(state.available_sources().count(), 1);
        assert!(state.idle());
    }

    #[test]
    fn an_empty_document_is_an_idle_empty_state() {
        let state = UpdateState::parse("{}").unwrap();
        assert!(state.idle());
        assert!(state.sources.is_empty());
    }

    #[test]
    fn busy_or_checking_is_not_idle() {
        assert!(!UpdateState::parse(r#"{"busy":true}"#).unwrap().idle());
        assert!(!UpdateState::parse(r#"{"checking":true}"#).unwrap().idle());
    }

    #[test]
    fn rejects_text_that_is_not_json() {
        assert!(UpdateState::parse("not json").is_err());
        assert!(UpdateState::parse(r#"{"busy":"yes"}"#).is_err());
    }
}
