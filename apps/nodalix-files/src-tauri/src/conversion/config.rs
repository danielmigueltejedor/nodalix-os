use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ConversionConfig {
    pub ask_on_extension_change: bool,
    pub keep_original: bool,
    pub allow_external_tools: bool,
    pub tools: ToolPaths,
}

#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub imagemagick: String,
    pub libreoffice: String,
    pub rsvg_convert: String,
    pub pdftoppm: String,
    pub heif_convert: String,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            ask_on_extension_change: true,
            keep_original: true,
            allow_external_tools: true,
            tools: ToolPaths {
                imagemagick: "magick".into(),
                libreoffice: "libreoffice".into(),
                rsvg_convert: "rsvg-convert".into(),
                pdftoppm: "pdftoppm".into(),
                heif_convert: "heif-convert".into(),
            },
        }
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/nodalix-files/conversions.toml"))
}

fn parse_bool(table: &toml::Table, key: &str, default: bool) -> bool {
    table
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn parse_tool(table: &toml::Table, key: &str, default: &str) -> String {
    table
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default.to_string())
}

pub fn load_conversion_config() -> ConversionConfig {
    let mut config = ConversionConfig::default();
    let Some(path) = config_path() else {
        return config;
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        crate::platform::debug_log(&format!(
            "conversion config: no file at {}",
            path.display()
        ));
        return config;
    };
    let parsed: toml::Table = match raw.parse() {
        Ok(toml::Value::Table(table)) => table,
        _ => {
            crate::platform::debug_log("conversion config: invalid toml root");
            return config;
        }
    };
    if let Some(conversion) = parsed.get("conversion").and_then(|v| v.as_table()) {
        config.ask_on_extension_change =
            parse_bool(conversion, "ask_on_extension_change", config.ask_on_extension_change);
        config.keep_original = parse_bool(conversion, "keep_original", config.keep_original);
        config.allow_external_tools =
            parse_bool(conversion, "allow_external_tools", config.allow_external_tools);
    }
    if let Some(tools) = parsed.get("conversion.tools").and_then(|v| v.as_table()) {
        config.tools.imagemagick =
            parse_tool(tools, "imagemagick", &config.tools.imagemagick);
        config.tools.libreoffice =
            parse_tool(tools, "libreoffice", &config.tools.libreoffice);
        config.tools.rsvg_convert =
            parse_tool(tools, "rsvg_convert", &config.tools.rsvg_convert);
        config.tools.pdftoppm = parse_tool(tools, "pdftoppm", &config.tools.pdftoppm);
        config.tools.heif_convert =
            parse_tool(tools, "heif_convert", &config.tools.heif_convert);
    }
    crate::platform::debug_log(&format!(
        "conversion config loaded from {}",
        path.display()
    ));
    config
}

pub fn tool_overrides(config: &ConversionConfig) -> HashMap<&'static str, String> {
    HashMap::from([
        ("magick", config.tools.imagemagick.clone()),
        ("libreoffice", config.tools.libreoffice.clone()),
        ("rsvg-convert", config.tools.rsvg_convert.clone()),
        ("pdftoppm", config.tools.pdftoppm.clone()),
        ("heif-convert", config.tools.heif_convert.clone()),
    ])
}
