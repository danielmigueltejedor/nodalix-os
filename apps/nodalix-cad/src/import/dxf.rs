use crate::{
    document::{Document, Entity},
    geometry::Point,
};
use std::{fs, path::Path};

use super::ImportSummary;

pub fn import_dxf(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let pairs = group_pairs(&data);
    let mut imported = 0usize;
    let mut i = 0usize;
    while i + 1 < pairs.len() {
        if pairs[i].0 == "0" && pairs[i].1 == "LINE" {
            let mut start = Point::default();
            let mut end = Point::default();
            i += 1;
            while i + 1 < pairs.len() && !(pairs[i].0 == "0" && !pairs[i].1.is_empty()) {
                match pairs[i].0.as_str() {
                    "10" => start.x = pairs[i].1.parse().unwrap_or(0.0),
                    "20" => start.y = pairs[i].1.parse().unwrap_or(0.0),
                    "11" => end.x = pairs[i].1.parse().unwrap_or(0.0),
                    "21" => end.y = pairs[i].1.parse().unwrap_or(0.0),
                    _ => {}
                }
                i += 1;
            }
            document.add_entity(Entity::Line {
                id: document.next_id(),
                layer: "Default".to_string(),
                start,
                end,
            });
            imported += 1;
            continue;
        }
        i += 1;
    }

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("drawing.dxf")
            .to_string(),
        format: "DXF".to_string(),
        summary: "Basic DXF import completed. Currently imports LINE entities.".to_string(),
        details: vec![("Imported entities".to_string(), imported.to_string())],
        warnings: vec!["DXF import is intentionally limited in this early version.".to_string()],
    })
}

fn group_pairs(data: &str) -> Vec<(String, String)> {
    let mut lines = data.lines();
    let mut pairs = Vec::new();
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        pairs.push((code.trim().to_string(), value.trim().to_string()));
    }
    pairs
}
