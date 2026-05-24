use crate::document::Document;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn export_via_dxf(document: &Document, target: &Path) -> Result<(), String> {
    let intermediate = intermediate_dxf_path(target);
    crate::export::dxf::export(document, &intermediate)?;

    if let Some(converter) = detected_converter() {
        return Err(format!(
            "DWG export bridge found `{converter}`, but automatic conversion is not wired yet. DXF intermediate was written to {}.",
            intermediate.display()
        ));
    }

    Err(format!(
        "DWG export needs an optional DWG converter. DXF intermediate was written to {}. Install ODA File Converter or LibreDWG, then convert this DXF to DWG.",
        intermediate.display()
    ))
}

fn intermediate_dxf_path(target: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let stem = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("nodecad-export");
    std::env::temp_dir().join(format!("{stem}-{stamp}.dxf"))
}

fn detected_converter() -> Option<String> {
    for command in ["ODAFileConverter", "dwg2dxf"] {
        if command_exists(command) {
            return Some(command.to_string());
        }
    }
    None
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
