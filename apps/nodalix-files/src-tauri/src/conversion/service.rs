use super::backends::FileConversionService;
use super::config::load_conversion_config;
use super::registry::{
    extensions_compatible_for_prompt, family_for_ext, normalize_ext, ConversionFamily,
    ConversionRegistry,
};
use super::registry::ConversionCapability;
use super::tools::tool_map_for_frontend;
use crate::platform;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static SERVICE: OnceLock<Mutex<FileConversionService>> = OnceLock::new();

fn service() -> &'static Mutex<FileConversionService> {
    SERVICE.get_or_init(|| {
        let config = load_conversion_config();
        Mutex::new(FileConversionService::new(config))
    })
}

pub fn init_conversion_backends() {
    let _ = service().lock().expect("conversion service lock");
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenameConversionScenario {
    NoExtensionChange,
    ConvertAvailable,
    Incompatible,
    Unavailable,
    CategoryUnavailable,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenameConversionPreview {
    pub old_path: String,
    pub old_name: String,
    pub new_name: String,
    pub dest_path: String,
    pub dest_exists: bool,
    pub extension_changed: bool,
    pub from_ext: Option<String>,
    pub to_ext: Option<String>,
    pub scenario: RenameConversionScenario,
    pub capability: Option<ConversionCapability>,
    pub message: Option<String>,
    pub ask_on_extension_change: bool,
    pub keep_original: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConversionSettings {
    pub ask_on_extension_change: bool,
    pub keep_original: bool,
    pub allow_external_tools: bool,
    pub tools_available: std::collections::HashMap<String, bool>,
}

pub fn get_conversion_settings() -> ConversionSettings {
    let guard = service().lock().expect("conversion service lock");
    let config = guard.config().clone();
    let tools = guard.tools().clone();
    ConversionSettings {
        ask_on_extension_change: config.ask_on_extension_change,
        keep_original: config.keep_original,
        allow_external_tools: config.allow_external_tools,
        tools_available: tool_map_for_frontend(&tools),
    }
}

pub fn preview_rename_conversion(old_path: &str, new_name: &str) -> Result<RenameConversionPreview, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("El nuevo nombre no puede estar vacío".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("El nuevo nombre no puede contener separadores de ruta".into());
    }

    let source = PathBuf::from(old_path);
    if !source.exists() {
        return Err(format!("La ruta no existe: {old_path}"));
    }
    let meta = std::fs::symlink_metadata(&source).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        return simple_preview(old_path, trimmed, RenameConversionScenario::NoExtensionChange, false);
    }

    let old_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let from_ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext);
    let dest = source
        .parent()
        .ok_or_else(|| "No se puede resolver la carpeta padre".to_string())?
        .join(trimmed);
    let to_ext = dest.extension().and_then(|e| e.to_str()).map(normalize_ext);
    let extension_changed = from_ext != to_ext;
    let dest_exists = dest.exists();
    let dest_path = path_to_string(&dest);

    if !extension_changed {
        return Ok(RenameConversionPreview {
            old_path: old_path.to_string(),
            old_name,
            new_name: trimmed.to_string(),
            dest_path,
            dest_exists,
            extension_changed: false,
            from_ext,
            to_ext,
            scenario: RenameConversionScenario::NoExtensionChange,
            capability: None,
            message: None,
            ask_on_extension_change: service()
                .lock()
                .expect("conversion service lock")
                .config()
                .ask_on_extension_change,
            keep_original: service()
                .lock()
                .expect("conversion service lock")
                .config()
                .keep_original,
        });
    }

    let guard = service().lock().expect("conversion service lock");
    let config = guard.config().clone();
    let tools = guard.tools().clone();
    drop(guard);

    let from = from_ext.clone().unwrap_or_default();
    let to = to_ext.clone().unwrap_or_default();

    if let Some(mut capability) = ConversionRegistry::lookup(&from, &to, &tools, &config) {
        if !capability.available {
            return Ok(RenameConversionPreview {
                old_path: old_path.to_string(),
                old_name,
                new_name: trimmed.to_string(),
                dest_path,
                dest_exists,
                extension_changed: true,
                from_ext,
                to_ext,
                scenario: RenameConversionScenario::Unavailable,
                capability: Some(capability.clone()),
                message: capability.unavailable_reason.clone(),
                ask_on_extension_change: config.ask_on_extension_change,
                keep_original: config.keep_original,
            });
        }
        return Ok(RenameConversionPreview {
            old_path: old_path.to_string(),
            old_name,
            new_name: trimmed.to_string(),
            dest_path,
            dest_exists,
            extension_changed: true,
            from_ext,
            to_ext,
            scenario: RenameConversionScenario::ConvertAvailable,
            capability: Some(capability),
            message: None,
            ask_on_extension_change: config.ask_on_extension_change,
            keep_original: config.keep_original,
        });
    }

    if family_for_ext(&from) == ConversionFamily::VideoAudio
        || family_for_ext(&to) == ConversionFamily::VideoAudio
    {
        return Ok(RenameConversionPreview {
            old_path: old_path.to_string(),
            old_name,
            new_name: trimmed.to_string(),
            dest_path,
            dest_exists,
            extension_changed: true,
            from_ext,
            to_ext,
            scenario: RenameConversionScenario::CategoryUnavailable,
            capability: None,
            message: Some(ConversionRegistry::incompatible_message(&from, &to)),
            ask_on_extension_change: config.ask_on_extension_change,
            keep_original: config.keep_original,
        });
    }

    let compatible = extensions_compatible_for_prompt(&from, &to);
    Ok(RenameConversionPreview {
        old_path: old_path.to_string(),
        old_name,
        new_name: trimmed.to_string(),
        dest_path,
        dest_exists,
        extension_changed: true,
        from_ext,
        to_ext,
        scenario: if compatible {
            RenameConversionScenario::Incompatible
        } else {
            RenameConversionScenario::CategoryUnavailable
        },
        capability: None,
        message: Some(ConversionRegistry::incompatible_message(&from, &to)),
        ask_on_extension_change: config.ask_on_extension_change,
        keep_original: config.keep_original,
    })
}

fn simple_preview(
    old_path: &str,
    new_name: &str,
    scenario: RenameConversionScenario,
    extension_changed: bool,
) -> Result<RenameConversionPreview, String> {
    let source = PathBuf::from(old_path);
    let dest = source.parent().unwrap().join(new_name);
    let config = service().lock().expect("conversion service lock").config().clone();
    Ok(RenameConversionPreview {
        old_path: old_path.to_string(),
        old_name: source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        new_name: new_name.to_string(),
        dest_path: path_to_string(&dest),
        dest_exists: dest.exists(),
        extension_changed,
        from_ext: source
            .extension()
            .and_then(|e| e.to_str())
            .map(normalize_ext),
        to_ext: dest.extension().and_then(|e| e.to_str()).map(normalize_ext),
        scenario,
        capability: None,
        message: None,
        ask_on_extension_change: config.ask_on_extension_change,
        keep_original: config.keep_original,
    })
}

pub fn convert_file(source_path: &str, dest_path: &str) -> Result<String, String> {
    let source = PathBuf::from(source_path);
    let dest = PathBuf::from(dest_path);
    if !source.is_file() {
        return Err(format!("No es un archivo: {source_path}"));
    }
    let from_ext = source
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "El archivo de origen no tiene extensión".to_string())?;
    let to_ext = dest
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "El destino debe tener extensión".to_string())?;

    let guard = service().lock().expect("conversion service lock");
    guard.convert(&source, &dest, from_ext, to_ext)?;
    Ok(path_to_string(&dest))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn preview_txt_to_md_offers_conversion() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        fs::write(&file, b"hello").unwrap();
        let preview = preview_rename_conversion(
            &path_to_string(&file),
            "notes.md",
        )
        .unwrap();
        assert!(preview.extension_changed);
        assert!(matches!(
            preview.scenario,
            RenameConversionScenario::ConvertAvailable
        ));
        assert!(preview.capability.as_ref().unwrap().available);
    }

    #[test]
    fn preview_mp4_to_mp3_category_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("clip.mp4");
        fs::write(&file, b"fake").unwrap();
        let preview = preview_rename_conversion(&path_to_string(&file), "clip.mp3").unwrap();
        assert!(matches!(
            preview.scenario,
            RenameConversionScenario::CategoryUnavailable
        ));
    }

    #[test]
    fn preview_same_extension_no_prompt() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, b"x").unwrap();
        let preview = preview_rename_conversion(&path_to_string(&file), "b.txt").unwrap();
        assert!(!preview.extension_changed);
        assert!(matches!(
            preview.scenario,
            RenameConversionScenario::NoExtensionChange
        ));
    }

    #[test]
    fn convert_png_to_jpg_keeps_source() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("pixel.png");
        let dst = dir.path().join("pixel.jpg");
        let mut buffer = std::io::Cursor::new(Vec::new());
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        fs::write(&src, buffer.into_inner()).unwrap();
        convert_file(&path_to_string(&src), &path_to_string(&dst)).unwrap();
        assert!(src.exists());
        assert!(dst.exists());
    }
}
