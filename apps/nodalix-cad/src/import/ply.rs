use crate::document::{Document, ImportedReference};
use std::path::Path;

use super::ImportSummary;

pub fn import_ply_placeholder(
    path: &Path,
    document: &mut Document,
) -> Result<ImportSummary, String> {
    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "PLY".to_string(),
        summary: "PLY reference loaded. Geometry parsing is planned.".to_string(),
        warnings: vec!["PLY parsing is not implemented yet.".to_string()],
    });
    document.modified = true;
    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("mesh.ply")
            .to_string(),
        format: "PLY reference".to_string(),
        summary: "PLY stored as external reference placeholder.".to_string(),
        details: Vec::new(),
        warnings: vec!["PLY geometry parsing is planned.".to_string()],
    })
}
