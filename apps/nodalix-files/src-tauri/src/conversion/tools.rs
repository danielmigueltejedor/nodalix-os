use super::config::{tool_overrides, ConversionConfig};
use crate::platform;
use serde::Serialize;
use std::collections::HashMap;
use std::{env, fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug, Default, Serialize)]
pub struct ToolAvailability {
    pub imagemagick: bool,
    pub convert_legacy: bool,
    pub libreoffice: bool,
    pub rsvg_convert: bool,
    pub pdftoppm: bool,
    pub heif_convert: bool,
}

impl ToolAvailability {
    pub fn default_all_available() -> Self {
        Self {
            imagemagick: true,
            convert_legacy: true,
            libreoffice: true,
            rsvg_convert: true,
            pdftoppm: true,
            heif_convert: true,
        }
    }
}

fn command_exists(program: &str) -> bool {
    let program = program.trim();
    if program.is_empty() {
        return false;
    }
    if program.contains('/') {
        return is_executable(Path::new(program));
    }
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|dir| is_executable(&dir.join(program))))
        .unwrap_or(false)
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn detect_tools(config: &ConversionConfig) -> ToolAvailability {
    let overrides = tool_overrides(config);
    let imagemagick = command_exists(
        overrides
            .get("magick")
            .map(String::as_str)
            .unwrap_or("magick"),
    );
    let convert_legacy = command_exists("convert");
    let libreoffice = command_exists(
        overrides
            .get("libreoffice")
            .map(String::as_str)
            .unwrap_or("libreoffice"),
    );
    let rsvg_convert = command_exists(
        overrides
            .get("rsvg-convert")
            .map(String::as_str)
            .unwrap_or("rsvg-convert"),
    );
    let pdftoppm = command_exists(
        overrides
            .get("pdftoppm")
            .map(String::as_str)
            .unwrap_or("pdftoppm"),
    );
    let heif_convert = command_exists(
        overrides
            .get("heif-convert")
            .map(String::as_str)
            .unwrap_or("heif-convert"),
    );

    let tools = ToolAvailability {
        imagemagick,
        convert_legacy,
        libreoffice,
        rsvg_convert,
        pdftoppm,
        heif_convert,
    };
    log_tools(&tools);
    tools
}

fn log_tools(tools: &ToolAvailability) {
    platform::debug_log(&format!(
        "conversion backends: magick={} convert={} libreoffice={} rsvg-convert={} pdftoppm={} heif-convert={}",
        tools.imagemagick,
        tools.convert_legacy,
        tools.libreoffice,
        tools.rsvg_convert,
        tools.pdftoppm,
        tools.heif_convert
    ));
}

pub fn resolved_program(config: &ConversionConfig, key: &str, fallback: &str) -> String {
    tool_overrides(config)
        .get(key)
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

pub fn tool_map_for_frontend(tools: &ToolAvailability) -> HashMap<String, bool> {
    HashMap::from([
        ("magick".into(), tools.imagemagick),
        ("convert".into(), tools.convert_legacy),
        ("libreoffice".into(), tools.libreoffice),
        ("rsvg-convert".into(), tools.rsvg_convert),
        ("pdftoppm".into(), tools.pdftoppm),
        ("heif-convert".into(), tools.heif_convert),
    ])
}
