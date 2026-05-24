use crate::document::{Document, ImportedReference};
use std::path::Path;

use super::ImportSummary;

pub fn import_obj_placeholder(
    path: &Path,
    document: &mut Document,
) -> Result<ImportSummary, String> {
    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "OBJ".to_string(),
        summary: "OBJ reference loaded. Geometry parsing is planned.".to_string(),
        warnings: vec!["OBJ parsing is not implemented yet.".to_string()],
    });
    document.modified = true;
    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("model.obj")
            .to_string(),
        format: "OBJ reference".to_string(),
        summary: "OBJ stored as external reference placeholder.".to_string(),
        details: Vec::new(),
        warnings: vec!["OBJ geometry parsing is planned.".to_string()],
    })
}
