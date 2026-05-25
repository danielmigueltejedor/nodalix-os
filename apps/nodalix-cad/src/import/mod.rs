pub mod dwg;
pub mod dxf;
pub mod native;
pub mod obj;
pub mod ply;
pub mod step;
pub mod stl;

use crate::document::Document;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ImportSummary {
    pub file_name: String,
    pub format: String,
    pub summary: String,
    pub details: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

pub fn import_path(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "step" | "stp" | "iges" | "igs" => step::import_step_metadata(path, document),
        "stl" => stl::import_stl(path, document),
        "dxf" => dxf::import_dxf(path, document),
        "dwg" => dwg::import_dwg(path, document),
        "nodcad" | "lixcad" => {
            *document = native::open(path)?;
            Ok(ImportSummary {
                file_name: path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("project.nodcad")
                    .to_string(),
                format: "NODCAD".to_string(),
                summary: "Native Nodalix CAD document opened".to_string(),
                details: vec![("Entities".to_string(), document.entities.len().to_string())],
                warnings: Vec::new(),
            })
        }
        "obj" => obj::import_obj_placeholder(path, document),
        "ply" => ply::import_ply_placeholder(path, document),
        _ => Err(format!("Unsupported import format: .{ext}")),
    }
}
