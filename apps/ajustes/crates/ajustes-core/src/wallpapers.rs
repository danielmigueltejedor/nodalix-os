//! What the session service reports for the Backgrounds page: the animated catalog
//! (`GetWallpapers`, a JSON array) and the player's state (`GetWallpaperState`). Read as
//! forgivingly as the other service documents: a missing field is empty, an unknown one ignored.

use serde::Deserialize;

/// How the player pauses, in the order of the service's `pause-on-maximize-or-fullscreen`.
pub const PAUSE_CHOICES: [&str; 3] = [
    "Never pause",
    "Pause when a screen is covered",
    "Pause when all screens are covered",
];

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Wallpaper {
    pub title: String,
    pub path: String,
    /// A still image to show in the gallery instead of playing the video.
    pub preview: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub unreadable: bool,
    pub favorite: bool,
}

impl Wallpaper {
    /// `Title · 1920 × 1080 · 30 FPS`, or the title alone when the size is not known.
    pub fn details(&self) -> String {
        if self.unreadable {
            return format!("{} · ✗", self.title);
        }
        if self.width == 0 {
            return self.title.clone();
        }
        format!(
            "{} · {} × {} · {} FPS",
            self.title,
            self.width,
            self.height,
            self.fps.round() as i64
        )
    }
}

pub fn parse_catalog(json: &str) -> Result<Vec<Wallpaper>, serde_json::Error> {
    serde_json::from_str(json)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Options {
    #[serde(rename = "pause-on-maximize-or-fullscreen")]
    pub pause: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct PlayerState {
    /// The animated background player is installed.
    pub available: bool,
    /// An animated background is the one in use.
    pub requested: bool,
    pub path: String,
    pub message: String,
    pub options: Options,
}

impl PlayerState {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Whether `path` is the animated background in use.
    pub fn is_active(&self, path: &str) -> bool {
        self.requested && self.path == path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catalog_item_is_read_with_its_size() {
        let list = parse_catalog(
            r#"[{"title":"Aurora","path":"/a.mp4","preview":"/a.jpg","width":1920,"height":1080,"fps":29.97,"extra":1}]"#,
        )
        .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].details(), "Aurora · 1920 × 1080 · 30 FPS");
    }

    #[test]
    fn an_item_without_size_shows_only_its_title() {
        let list = parse_catalog(r#"[{"title":"Lluvia","path":"/l.mp4"}]"#).unwrap();
        assert_eq!(list[0].details(), "Lluvia");
    }

    #[test]
    fn the_player_state_marks_the_active_wallpaper() {
        let state = PlayerState::parse(
            r#"{"available":true,"requested":true,"path":"/a.mp4","message":"ok","options":{"pause-on-maximize-or-fullscreen":2}}"#,
        )
        .unwrap();
        assert!(state.is_active("/a.mp4"));
        assert!(!state.is_active("/b.mp4"));
        assert_eq!(state.options.pause, Some(2));
        let idle = PlayerState::parse(r#"{"available":true,"path":"/a.mp4"}"#).unwrap();
        assert!(!idle.is_active("/a.mp4"));
    }

    #[test]
    fn not_json_is_an_error_not_a_panic() {
        assert!(parse_catalog("nope").is_err());
        assert!(PlayerState::parse("").is_err());
    }
}
