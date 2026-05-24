use crate::document::{Document, ImportedReference};
use std::{collections::BTreeMap, fs, path::Path};

use super::ImportSummary;

pub fn import_step_metadata(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let upper = data.to_ascii_uppercase();
    if !upper.contains("ISO-10303-21") || !upper.contains("HEADER") || !upper.contains("DATA") {
        return Err("File does not look like an ISO-10303 STEP exchange file".to_string());
    }

    let description =
        extract_call(&data, "FILE_DESCRIPTION").unwrap_or_else(|| "unknown".to_string());
    let file_name = extract_call(&data, "FILE_NAME").unwrap_or_else(|| "unknown".to_string());
    let schema = extract_call(&data, "FILE_SCHEMA").unwrap_or_else(|| "unknown".to_string());
    let entity_counts = entity_counts(&data);
    let top_entities = entity_counts
        .iter()
        .take(12)
        .map(|(name, count)| format!("{name}: {count}"))
        .collect::<Vec<_>>()
        .join(", ");

    let summary =
        "STEP metadata/reference loaded. B-Rep tessellation is not implemented yet.".to_string();
    let warnings = vec![
        "STEP geometry is stored as an external reference only.".to_string(),
        "Future tessellation should use OpenCascade/OCCT or a conversion bridge.".to_string(),
    ];
    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "STEP".to_string(),
        summary: summary.clone(),
        warnings: warnings.clone(),
    });
    document.modified = true;

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("model.step")
            .to_string(),
        format: "STEP metadata/reference".to_string(),
        summary,
        details: vec![
            ("FILE_DESCRIPTION".to_string(), description),
            ("FILE_NAME".to_string(), file_name),
            ("FILE_SCHEMA".to_string(), schema),
            ("Entity types".to_string(), entity_counts.len().to_string()),
            ("Top entities".to_string(), top_entities),
        ],
        warnings,
    })
}

fn extract_call(data: &str, name: &str) -> Option<String> {
    let upper = data.to_ascii_uppercase();
    let start = upper.find(name)?;
    let after = &data[start + name.len()..];
    let open = after.find('(')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut previous = '\0';
    for (idx, ch) in after[open..].char_indices() {
        if ch == '\'' && previous != '\\' {
            in_string = !in_string;
        }
        if !in_string {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(after[open + 1..open + idx].trim().to_string());
                }
            }
        }
        previous = ch;
    }
    None
}

fn entity_counts(data: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for line in data.lines() {
        let Some((_, rhs)) = line.split_once('=') else {
            continue;
        };
        let name = rhs
            .trim()
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>()
            .to_ascii_uppercase();
        if !name.is_empty() {
            *counts.entry(name).or_insert(0) += 1;
        }
    }
    counts
}
