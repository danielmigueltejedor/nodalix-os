use crate::document::Document;
use std::path::Path;

pub fn open(path: &Path) -> Result<Document, String> {
    Document::open_nodcad(path)
}
