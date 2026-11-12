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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub view: View,
    pub sort: SortKey,
    pub descending: bool,
    pub show_hidden: bool,
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
        };
        assert_eq!(Settings::parse(&settings.to_text()), settings);
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
