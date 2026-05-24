use crate::document::{Document, ImportedReference};
use std::{fs::File, io::Read, path::Path, process::Command};

use super::ImportSummary;

const DWG_SIGNATURES: &[(&[u8], &str)] = &[
    (b"AC1009", "AutoCAD R12"),
    (b"AC1012", "AutoCAD R13"),
    (b"AC1014", "AutoCAD R14"),
    (b"AC1015", "AutoCAD 2000"),
    (b"AC1018", "AutoCAD 2004"),
    (b"AC1021", "AutoCAD 2007"),
    (b"AC1024", "AutoCAD 2010"),
    (b"AC1027", "AutoCAD 2013"),
    (b"AC1032", "AutoCAD 2018"),
];

pub fn import_dwg_reference(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    let mut header = [0_u8; 64];
    let read = file.read(&mut header).map_err(|err| err.to_string())?;
    let signature = detect_signature(&header[..read]);
    let warnings = dwg_warnings(signature);
    let summary = match signature {
        Some((code, version)) => format!("DWG reference loaded ({code}, {version})."),
        None => "DWG reference loaded, but the AutoCAD signature was not recognized.".to_string(),
    };

    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "DWG".to_string(),
        summary: summary.clone(),
        warnings: warnings.clone(),
    });
    document.modified = true;

    let mut details = vec![
        (
            "DWG handling".to_string(),
            "Reference import only; geometry conversion requires an optional external DWG reader."
                .to_string(),
        ),
        (
            "External converter".to_string(),
            detected_converter().unwrap_or_else(|| {
                "not found: install ODA File Converter or LibreDWG tools".into()
            }),
        ),
    ];
    if let Some((code, version)) = signature {
        details.push(("Signature".to_string(), code.to_string()));
        details.push(("Detected version".to_string(), version.to_string()));
    }

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("drawing.dwg")
            .to_string(),
        format: "DWG".to_string(),
        summary,
        details,
        warnings,
    })
}

fn detect_signature(header: &[u8]) -> Option<(&'static str, &'static str)> {
    DWG_SIGNATURES
        .iter()
        .find(|(signature, _)| header.starts_with(signature))
        .map(|(signature, version)| (std::str::from_utf8(signature).unwrap_or("DWG"), *version))
}

fn dwg_warnings(signature: Option<(&str, &str)>) -> Vec<String> {
    let mut warnings = vec![
        "DWG is a proprietary binary CAD format; NodeCad does not yet parse DWG geometry natively."
            .to_string(),
        "Use this import as an external reference for now, or convert DWG to DXF with an optional converter."
            .to_string(),
    ];
    if signature.is_none() {
        warnings.push("File does not start with a known DWG signature.".to_string());
    }
    warnings
}

fn detected_converter() -> Option<String> {
    for command in ["ODAFileConverter", "dwgread", "dwg2dxf"] {
        if command_exists(command) {
            return Some(command.to_string());
        }
    }
    None
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
