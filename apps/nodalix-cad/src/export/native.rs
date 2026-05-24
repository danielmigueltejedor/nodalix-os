use crate::document::Document;
use std::path::Path;

pub fn save(document: &mut Document, path: &Path) -> Result<(), String> {
    document.save_nodcad(path)
}
