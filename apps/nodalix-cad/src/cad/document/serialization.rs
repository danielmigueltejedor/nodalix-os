use super::cad_document::CADDocument;
use std::{fs, path::Path};

pub const CAD_DOCUMENT_FORMAT_VERSION: u32 = 2;

/// Native CAD JSON format (`.lixcad` v2 schema). Legacy v1 files remain supported via
/// `crate::document::Document` and the adapter in `adapter.rs`.
pub fn save_cad_document(document: &CADDocument, path: &Path) -> Result<(), String> {
    let data = serde_json::to_string_pretty(document).map_err(|err| err.to_string())?;
    fs::write(path, data).map_err(|err| err.to_string())
}

pub fn load_cad_document(path: &Path) -> Result<CADDocument, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let doc: CADDocument = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(doc)
}
