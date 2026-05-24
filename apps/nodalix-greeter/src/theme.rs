use serde::Deserialize;
use std::{fs, path::Path};

pub const CONFIG_PATH: &str = "/etc/nodalix/greeter/config.toml";
pub const CSS_PATH: &str = "/usr/local/share/nodalix-greeter/nodalix-greeter.css";

#[derive(Clone, Debug, Deserialize)]
pub struct GreeterConfig {
    pub background: String,
    pub session_command: String,
    pub accent_color: String,
    pub font_family: String,
    pub default_user: String,
}

impl Default for GreeterConfig {
    fn default() -> Self {
        Self {
            background: "/etc/nodalix/wallpapers/current/greeter-wallpaper.png".to_string(),
            session_command: "uwsm start hyprland-uwsm.desktop".to_string(),
            accent_color: "#cba6f7".to_string(),
            font_family: "JetBrainsMono Nerd Font".to_string(),
            default_user: String::new(),
        }
    }
}

impl GreeterConfig {
    pub fn load_default() -> Self {
        Self::load_from(CONFIG_PATH)
    }

    pub fn load_from(path: impl AsRef<Path>) -> Self {
        let defaults = Self::default();
        let Ok(raw) = fs::read_to_string(path) else {
            return defaults;
        };

        match toml::from_str::<PartialGreeterConfig>(&raw) {
            Ok(partial) => partial.merge(defaults),
            Err(err) => {
                eprintln!("nodalix-greeter: ignoring invalid config: {err}");
                defaults
            }
        }
    }

    pub fn css_overrides(&self) -> String {
        format!(
            "@define-color nodalix_accent {}; .clock {{ font-family: '{}'; }}",
            self.accent_color,
            self.font_family.replace('\'', "")
        )
    }
}

#[derive(Debug, Deserialize)]
struct PartialGreeterConfig {
    background: Option<String>,
    session_command: Option<String>,
    accent_color: Option<String>,
    font_family: Option<String>,
    default_user: Option<String>,
}

impl PartialGreeterConfig {
    fn merge(self, defaults: GreeterConfig) -> GreeterConfig {
        GreeterConfig {
            background: self.background.unwrap_or(defaults.background),
            session_command: self.session_command.unwrap_or(defaults.session_command),
            accent_color: self.accent_color.unwrap_or(defaults.accent_color),
            font_family: self.font_family.unwrap_or(defaults.font_family),
            default_user: self.default_user.unwrap_or(defaults.default_user),
        }
    }
}

pub fn load_css() -> String {
    fs::read_to_string(CSS_PATH)
        .unwrap_or_else(|_| include_str!("../data/nodalix-greeter.css").to_string())
}
