//! What Archivador remembers between runs: how folders are shown.

use serde::{Deserialize, Serialize};

use crate::entry::SortKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    List,
    Grid,
    /// A column per folder on the way, the way Finder shows them.
    Columns,
}

/// How Archivador is drawn, when the user does not want the system's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Follow the system.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub view: View,
    pub sort: SortKey,
    pub descending: bool,
    pub show_hidden: bool,
    pub appearance: Appearance,
    /// Whether the file system (`/`) is listed among the devices in the sidebar.
    pub show_root: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            view: View::default(),
            sort: SortKey::default(),
            descending: false,
            show_hidden: false,
            appearance: Appearance::default(),
            show_root: true,
        }
    }
}

impl Settings {
    /// The settings in `text` (JSON). Anything unreadable, or a field that is missing or no longer
    /// valid, falls back to the default: a broken file must never keep the app from opening.
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn to_text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let settings = Settings {
            view: View::Grid,
            sort: SortKey::Size,
            descending: true,
            show_hidden: true,
            appearance: Appearance::Dark,
            show_root: false,
        };
        assert_eq!(Settings::parse(&settings.to_text()), settings);
    }

    #[test]
    fn the_file_system_is_listed_unless_the_user_says_otherwise() {
        assert!(Settings::default().show_root);
        assert!(Settings::parse(r#"{"view":"grid"}"#).show_root);
        assert!(!Settings::parse(r#"{"show_root":false}"#).show_root);
        assert_eq!(
            Settings::parse(r#"{"appearance":"light"}"#).appearance,
            Appearance::Light
        );
        assert_eq!(
            Settings::parse(r#"{"appearance":"neon"}"#),
            Settings::default()
        );
    }

    #[test]
    fn the_columns_view_is_remembered() {
        let text = Settings {
            view: View::Columns,
            ..Settings::default()
        }
        .to_text();
        assert!(text.contains("\"columns\""));
        assert_eq!(Settings::parse(&text).view, View::Columns);
    }

    #[test]
    fn missing_or_broken_input_gives_defaults() {
        assert_eq!(Settings::parse(""), Settings::default());
        assert_eq!(Settings::parse("not json"), Settings::default());
        assert_eq!(
            Settings::parse(r#"{"view":"grid"}"#),
            Settings {
                view: View::Grid,
                ..Settings::default()
            }
        );
        // An unknown sort key voids the whole file, not just that field.
        assert_eq!(Settings::parse(r#"{"sort":"colour"}"#), Settings::default());
    }
}
