use super::config::ConversionConfig;
use super::tools::ToolAvailability;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ConversionCapability {
    pub from_ext: String,
    pub to_ext: String,
    pub label: String,
    pub backend: String,
    pub lossless: bool,
    pub lossy: bool,
    pub requires_external: bool,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub warnings: Vec<String>,
    pub user_note: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversionFamily {
    Image,
    Text,
    Office,
    Vector,
    PdfRaster,
    Heic,
    VideoAudio,
    Unknown,
}

pub fn normalize_ext(ext: &str) -> String {
    match ext.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
        "jpeg" => "jpg".into(),
        "tif" => "tiff".into(),
        "yml" => "yaml".into(),
        "heif" => "heic".into(),
        "markdown" => "md".into(),
        _ => ext.trim().trim_start_matches('.').to_ascii_lowercase(),
    }
}

pub fn family_for_ext(ext: &str) -> ConversionFamily {
    let ext = normalize_ext(ext);
    match ext.as_str() {
        "png" | "jpg" | "webp" | "bmp" | "tiff" | "gif" | "ico" | "avif" => ConversionFamily::Image,
        "heic" => ConversionFamily::Heic,
        "txt" | "md" | "csv" | "json" | "toml" | "yaml" => ConversionFamily::Text,
        "docx" | "odt" | "pptx" | "xlsx" | ConversionFamily::Office,
        "svg" => ConversionFamily::Vector,
        "pdf" => ConversionFamily::PdfRaster,
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "mp3" | "flac" | "wav" | "ogg" | "m4a"
        | "aac" => ConversionFamily::VideoAudio,
        _ => ConversionFamily::Unknown,
    }
}

fn lossy_target(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "jpg" | "webp")
}

fn png_to_jpg_warnings() -> Vec<String> {
    vec![
        "JPEG no admite transparencia.".into(),
        "La conversión a JPEG puede reducir la calidad.".into(),
    ]
}

fn lossy_warnings(to_ext: &str) -> Vec<String> {
    if lossy_target(to_ext) {
        vec!["La conversión puede reducir la calidad.".into()]
    } else {
        Vec::new()
    }
}

pub struct ConversionRegistry;

impl ConversionRegistry {
    pub fn lookup(from_ext: &str, to_ext: &str, tools: &ToolAvailability, config: &ConversionConfig) -> Option<ConversionCapability> {
        let from = normalize_ext(from_ext);
        let to = normalize_ext(to_ext);
        if from == to {
            return None;
        }

        let pair = (from.as_str(), to.as_str());
        let mut cap = match pair {
            ("png", "jpg") => base_capability(&from, &to, "PNG → JPEG", "image (Rust)", false, true, false),
            ("jpg", "png") => base_capability(&from, &to, "JPEG → PNG", "image (Rust)", true, false, false),
            ("webp", "png") | ("webp", "jpg") => {
                base_capability(&from, &to, &format!("WebP → {}", to.to_uppercase()), "image (Rust)", pair == ("webp", "png"), pair == ("webp", "jpg"), false)
            }
            ("png", "webp") | ("jpg", "webp") => {
                base_capability(&from, &to, &format!("{} → WebP", from.to_uppercase()), "image (Rust)", false, true, false)
            }
            ("bmp", "png") => base_capability(&from, &to, "BMP → PNG", "image (Rust)", true, false, false),
            ("tiff", "png") => base_capability(&from, &to, "TIFF → PNG", "image (Rust)", true, false, false),
            ("txt", "md") => base_capability(&from, &to, "Texto → Markdown", "texto (interno)", true, false, false),
            ("md", "txt") => base_capability(&from, &to, "Markdown → Texto", "texto (interno)", true, false, false),
            ("csv", "txt") | ("json", "txt") | ("toml", "txt") | ("yaml", "txt") => {
                base_capability(&from, &to, &format!("{} → Texto", from.to_uppercase()), "texto (interno)", true, false, false)
            }
            ("docx", "pdf") | ("odt", "pdf") | ("pptx", "pdf") | ("xlsx", "pdf") => {
                base_capability(&from, &to, &format!("{} → PDF", from.to_uppercase()), "LibreOffice", true, false, true)
            }
            ("svg", "png") => base_capability(&from, &to, "SVG → PNG", "rsvg-convert", true, false, true),
            ("pdf", "png") => base_capability(&from, &to, "PDF → PNG", "pdftoppm", false, false, true),
            ("heic", "jpg") | ("heic", "png") => {
                base_capability(&from, &to, &format!("HEIC → {}", to.to_uppercase()), "ImageMagick/heif-convert", pair == ("heic", "png"), pair == ("heic", "jpg"), true)
            }
            _ => return None,
        };

        if cap.from_ext == "png" && cap.to_ext == "jpg" {
            cap.warnings = png_to_jpg_warnings();
        } else {
            cap.warnings = lossy_warnings(&cap.to_ext);
        }

        if cap.requires_external && !config.allow_external_tools {
            cap.available = false;
            cap.unavailable_reason = Some(
                "Las herramientas externas están desactivadas en la configuración.".into(),
            );
            return Some(cap);
        }

        cap.available = resolve_backend_available(&cap, tools);
        if !cap.available {
            cap.unavailable_reason = cap.unavailable_reason.or_else(|| {
                Some("No hay un conversor disponible en el sistema.".into())
            });
        }
        Some(cap)
    }

    pub fn incompatible_message(from_ext: &str, to_ext: &str) -> String {
        let from = normalize_ext(from_ext);
        let to = normalize_ext(to_ext);
        let from_family = family_for_ext(&from);
        let to_family = family_for_ext(&to);

        if from_family == ConversionFamily::VideoAudio || to_family == ConversionFamily::VideoAudio {
            return "Conversión no disponible todavía para vídeo/audio.".into();
        }

        format!(
            "No se puede convertir de .{from} a .{to}. Puedes renombrar igualmente, pero el formato interno no cambiará."
        )
    }
}

fn base_capability(
    from: &str,
    to: &str,
    label: &str,
    backend: &str,
    lossless: bool,
    lossy: bool,
    requires_external: bool,
) -> ConversionCapability {
    ConversionCapability {
        from_ext: from.to_string(),
        to_ext: to.to_string(),
        label: label.to_string(),
        backend: backend.to_string(),
        lossless,
        lossy,
        requires_external,
        available: !requires_external,
        unavailable_reason: None,
        warnings: Vec::new(),
        user_note: None,
    }
}

fn resolve_backend_available(cap: &ConversionCapability, tools: &ToolAvailability) -> bool {
    if !cap.requires_external {
        return true;
    }
    match cap.backend.as_str() {
        "LibreOffice" => tools.libreoffice,
        "rsvg-convert" => tools.rsvg_convert,
        "pdftoppm" => tools.pdftoppm,
        "ImageMagick/heif-convert" => tools.imagemagick || tools.heif_convert,
        _ => false,
    }
}

pub fn extensions_compatible_for_prompt(from_ext: &str, to_ext: &str) -> bool {
    let from = normalize_ext(from_ext);
    let to = normalize_ext(to_ext);
    if from == to {
        return false;
    }
    if ConversionRegistry::lookup(&from, &to, &ToolAvailability::default_all_available(), &ConversionConfig::default()).is_some() {
        return true;
    }
    family_for_ext(&from) == family_for_ext(&to)
        && family_for_ext(&from) != ConversionFamily::Unknown
}
