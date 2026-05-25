use crate::document::{Document, ImportedReference};
use std::{
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use super::ImportSummary;

pub const DWG_SETUP_REQUIRED: &str = "DWG_IMPORT_SETUP_REQUIRED";

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

#[derive(Clone, Debug)]
pub struct DwgBackend {
    pub name: &'static str,
    pub kind: DwgBackendKind,
    pub command: Option<PathBuf>,
    pub automatic: bool,
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DwgBackendKind {
    Oda,
    LibreDwg,
    FreeCad,
}

#[derive(Clone, Debug, Default)]
pub struct DwgImportConfig {
    pub path: Option<PathBuf>,
    pub backend: Option<String>,
    pub converter_path: Option<PathBuf>,
}

pub fn import_dwg(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    log_dwg(format!("DWG import requested for {}", path.display()));
    let signature = inspect_signature(path)?;
    let backends = detect_backends();
    for backend in &backends {
        log_backend_status(backend);
    }
    if let Some(backend) = backends
        .into_iter()
        .find(|backend| backend.command.is_some() && backend.automatic)
    {
        return convert_and_import(path, document, signature, backend);
    }

    let status = converter_status_text();
    let mut warnings = dwg_warnings(signature);
    warnings.push("No automatic DWG converter is configured yet.".to_string());
    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "DWG".to_string(),
        summary: dwg_reference_summary(signature),
        warnings: warnings.clone(),
    });
    document.modified = true;

    Err(format!(
        "{DWG_SETUP_REQUIRED}\nDWG import uses an external converter internally.\n\n{status}"
    ))
}

pub fn converter_status_text() -> String {
    let mut lines = Vec::new();
    let config = load_config();
    match &config.path {
        Some(path) => {
            lines.push(format!("DWG import config loaded from {}", path.display()));
            lines.push(format!(
                "backend={}",
                config.backend.as_deref().unwrap_or("not set")
            ));
            lines.push(format!(
                "converter_path={}",
                config
                    .converter_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "not set".to_string())
            ));
        }
        None => {
            lines.push(format!(
                "DWG import not configured. Expected {} with [dwg_import] backend and converter_path.",
                settings_path().display()
            ));
        }
    }
    lines.push("Detected DWG conversion backends:".to_string());
    for backend in detect_backends() {
        let state = backend
            .command
            .as_ref()
            .map(|path| {
                format!(
                    "found at {} · exists={} · executable={}",
                    path.display(),
                    path.exists(),
                    is_executable(path)
                )
            })
            .unwrap_or_else(|| "not found".to_string());
        let automatic = if backend.automatic {
            "automatic import available"
        } else {
            "detected/configuration only for now"
        };
        let source = backend.source.as_deref().unwrap_or("auto");
        lines.push(format!(
            "- {}: {state} ({automatic}; source={source})",
            backend.name
        ));
    }
    lines.push("\nConfig search paths:".to_string());
    for path in settings_paths() {
        lines.push(format!("- {}", path.display()));
    }
    lines.push(
        "Example:\n[dwg_import]\nbackend = \"oda\"\nconverter_path = \"/usr/bin/oda-file-converter\""
            .to_string(),
    );
    let text = lines.join("\n");
    log_dwg(text.clone());
    text
}

pub fn settings_path() -> PathBuf {
    home_dir().join("lixcad/settings.toml")
}

pub fn settings_paths() -> Vec<PathBuf> {
    vec![
        home_dir().join("lixcad/settings.toml"),
        home_dir().join(".config/lixcad/settings.toml"),
        home_dir().join(".config/nodalix-cad/settings.toml"),
    ]
}

pub fn dwg_status_report() -> String {
    let _ = validate_config();
    converter_status_text()
}

pub fn validate_config() -> DwgImportConfig {
    load_config()
}

fn convert_and_import(
    path: &Path,
    document: &mut Document,
    signature: Option<(&'static str, &'static str)>,
    backend: DwgBackend,
) -> Result<ImportSummary, String> {
    let workspace = import_workspace()?;
    fs::create_dir_all(&workspace).map_err(|err| err.to_string())?;
    let output = workspace.join(
        path.file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("drawing")
            .to_string()
            + ".dxf",
    );
    let log_path = workspace.join("conversion.log");
    let converter = backend
        .command
        .as_ref()
        .ok_or_else(|| "DWG converter was selected but has no executable path.".to_string())?;
    log_dwg(format!(
        "DWG conversion selected backend={} converter_path={} executable={}",
        backend.name,
        converter.display(),
        is_executable(converter)
    ));

    match backend.kind {
        DwgBackendKind::LibreDwg => {
            let output_arg = output.to_string_lossy().to_string();
            let input_arg = path.to_string_lossy().to_string();
            let args = ["-o".to_string(), output_arg, input_arg];
            log_dwg(format!(
                "running DWG converter command: {} {}",
                converter.display(),
                args.join(" ")
            ));
            let output_log = Command::new(converter)
                .args(args)
                .output()
                .map_err(|err| format!("Failed to run {}: {err}", converter.display()))?;
            write_conversion_log(&log_path, &output_log.stdout, &output_log.stderr)?;
            log_command_output(&backend, &output_log.stdout, &output_log.stderr);
            if !output_log.status.success() {
                return Err(format!(
                    "DWG conversion failed with {}. Log: {}",
                    backend.name,
                    log_path.display()
                ));
            }
        }
        DwgBackendKind::Oda => {
            let input_dir = path
                .parent()
                .ok_or_else(|| format!("DWG path has no parent directory: {}", path.display()))?;
            let filter = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("DWG path has no valid file name: {}", path.display()))?;
            let args = [
                input_dir.display().to_string(),
                workspace.display().to_string(),
                "ACAD2018".to_string(),
                "DXF".to_string(),
                "0".to_string(),
                "1".to_string(),
                filter.to_string(),
            ];
            log_dwg(format!(
                "running DWG converter command: {} {}",
                converter.display(),
                args.join(" ")
            ));
            let output_log = Command::new(converter)
                .args(args)
                .output()
                .map_err(|err| format!("Failed to run {}: {err}", converter.display()))?;
            write_conversion_log(&log_path, &output_log.stdout, &output_log.stderr)?;
            log_command_output(&backend, &output_log.stdout, &output_log.stderr);
            if !output_log.status.success() {
                return Err(format!(
                    "DWG conversion failed with {}. Log: {}",
                    backend.name,
                    log_path.display()
                ));
            }
        }
        DwgBackendKind::FreeCad => {
            return Err(format!(
                "{DWG_SETUP_REQUIRED}\n{} was detected, but automatic command wiring for this backend is not enabled yet.\n\n{}",
                backend.name,
                converter_status_text()
            ));
        }
    }

    let output = if output.exists() {
        output
    } else if let Some(found) = find_first_dxf(&workspace) {
        found
    } else {
        return Err(format!(
            "DWG conversion did not produce a DXF file. Expected: {}. Log: {}",
            output.display(),
            log_path.display()
        ));
    };

    let dxf_summary = crate::import::dxf::import_dxf(&output, document)
        .map_err(|err| format!("DWG converted to DXF, but DXF import failed: {err}"))?;
    let summary = dwg_reference_summary(signature);
    document.imported_references.push(ImportedReference {
        path: path.display().to_string(),
        format: "DWG".to_string(),
        summary: format!("DWG imported through {} -> DXF.", backend.name),
        warnings: dwg_warnings(signature),
    });
    document.modified = true;

    let mut details = vec![
        ("Backend".to_string(), backend.name.to_string()),
        ("Output DXF".to_string(), output.display().to_string()),
        ("Import log".to_string(), log_path.display().to_string()),
        ("Reference".to_string(), summary),
    ];
    details.extend(dxf_summary.details);

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("drawing.dwg")
            .to_string(),
        format: "DWG".to_string(),
        summary: "DWG converted internally to DXF and imported.".to_string(),
        details,
        warnings: dxf_summary.warnings,
    })
}

fn inspect_signature(path: &Path) -> Result<Option<(&'static str, &'static str)>, String> {
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    let mut header = [0_u8; 64];
    let read = file.read(&mut header).map_err(|err| err.to_string())?;
    Ok(detect_signature(&header[..read]))
}

fn dwg_reference_summary(signature: Option<(&str, &str)>) -> String {
    match signature {
        Some((code, version)) => format!("DWG reference loaded ({code}, {version})."),
        None => "DWG reference loaded, but the AutoCAD signature was not recognized.".to_string(),
    }
}

fn detect_signature(header: &[u8]) -> Option<(&'static str, &'static str)> {
    DWG_SIGNATURES
        .iter()
        .find(|(signature, _)| header.starts_with(signature))
        .map(|(signature, version)| (std::str::from_utf8(signature).unwrap_or("DWG"), *version))
}

fn dwg_warnings(signature: Option<(&str, &str)>) -> Vec<String> {
    let mut warnings = vec![
        "DWG is a proprietary binary CAD format; Lix CAD does not yet parse DWG geometry natively."
            .to_string(),
        "Use this import as an external reference for now, or convert DWG to DXF with an optional converter."
            .to_string(),
    ];
    if signature.is_none() {
        warnings.push("File does not start with a known DWG signature.".to_string());
    }
    warnings
}

fn detect_backends() -> Vec<DwgBackend> {
    let config = load_config();
    if let Some(path) = &config.path {
        log_dwg(format!("DWG import config loaded from {}", path.display()));
        log_dwg(format!(
            "backend={} converter_path={}",
            config.backend.as_deref().unwrap_or("not set"),
            config
                .converter_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not set".to_string())
        ));
    }
    let configured_backend = config.backend.as_deref();
    vec![
        make_backend(
            "ODA File Converter",
            DwgBackendKind::Oda,
            &[
                "oda-file-converter",
                "ODAFileConverter",
                "ODAFileConverterApp",
            ],
            &[
                PathBuf::from("/usr/bin/oda-file-converter"),
                PathBuf::from("/opt/oda-file-converter/oda-file-converter"),
            ],
            &config,
            configured_backend.map(backend_is_oda).unwrap_or(false),
        ),
        make_backend(
            "LibreDWG dwg2dxf",
            DwgBackendKind::LibreDwg,
            &["dwg2dxf"],
            &[],
            &config,
            configured_backend.map(backend_is_libredwg).unwrap_or(true),
        ),
        make_backend(
            "LibreDWG dwgread",
            DwgBackendKind::LibreDwg,
            &["dwgread"],
            &[],
            &config,
            false,
        ),
        make_backend(
            "FreeCAD",
            DwgBackendKind::FreeCad,
            &["freecadcmd", "FreeCADCmd", "freecad", "FreeCAD"],
            &[],
            &config,
            false,
        ),
    ]
}

fn make_backend(
    name: &'static str,
    kind: DwgBackendKind,
    commands: &[&str],
    fallback_paths: &[PathBuf],
    config: &DwgImportConfig,
    automatic: bool,
) -> DwgBackend {
    let configured_matches = match (&config.backend, kind) {
        (Some(backend), DwgBackendKind::Oda) => backend_is_oda(backend),
        (Some(backend), DwgBackendKind::LibreDwg) => backend_is_libredwg(backend),
        (Some(backend), DwgBackendKind::FreeCad) => backend_is_freecad(backend),
        (None, _) => false,
    };
    if configured_matches {
        if let Some(path) = &config.converter_path {
            return DwgBackend {
                name,
                kind,
                command: (path.exists() && is_executable(path))
                    .then(|| path.clone())
                    .or_else(|| {
                        log_dwg(format!(
                            "configured converter path is not usable: {} exists={} executable={}",
                            path.display(),
                            path.exists(),
                            is_executable(path)
                        ));
                        None
                    }),
                automatic,
                source: Some(format!(
                    "config {}",
                    config
                        .path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                )),
            };
        }
    }
    DwgBackend {
        name,
        kind,
        command: find_command(commands, fallback_paths),
        automatic,
        source: Some("path/fallback".to_string()),
    }
}

fn find_command(candidates: &[&str], fallback_paths: &[PathBuf]) -> Option<PathBuf> {
    for command in candidates {
        if let Some(path) = command_path(command) {
            return Some(path);
        }
        for root in [
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            home_dir().join(".local/bin"),
        ] {
            let candidate = root.join(command);
            if candidate.exists() && is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    for candidate in fallback_paths {
        if candidate.exists() && is_executable(candidate) {
            return Some(candidate.clone());
        }
    }
    None
}

fn command_path(command: &str) -> Option<PathBuf> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command}"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn load_config() -> DwgImportConfig {
    for path in settings_paths() {
        if let Ok(data) = fs::read_to_string(&path) {
            let mut config = parse_config_data(&data);
            config.path = Some(path);
            return config;
        }
    }
    DwgImportConfig::default()
}

fn parse_config_data(data: &str) -> DwgImportConfig {
    let mut config = DwgImportConfig::default();
    let mut in_dwg_import = false;
    for raw_line in data.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_dwg_import = line.trim_matches(&['[', ']'][..]).trim() == "dwg_import";
            continue;
        }
        if !in_dwg_import {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "backend" => config.backend = Some(value.to_ascii_lowercase()),
            "converter_path" => config.converter_path = Some(PathBuf::from(value)),
            _ => {}
        }
    }
    config
}

fn import_workspace() -> Result<PathBuf, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    Ok(home_dir()
        .join(".cache/lixcad/imports")
        .join(format!("{millis}")))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn write_conversion_log(path: &Path, stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    let mut data = String::new();
    data.push_str("stdout:\n");
    data.push_str(&String::from_utf8_lossy(stdout));
    data.push_str("\n\nstderr:\n");
    data.push_str(&String::from_utf8_lossy(stderr));
    fs::write(path, data).map_err(|err| err.to_string())
}

fn find_first_dxf(directory: &Path) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("dxf"))
                .unwrap_or(false)
        })
}

fn backend_is_oda(value: &str) -> bool {
    matches!(
        normalize_backend(value).as_str(),
        "oda" | "odafileconverter" | "odafileconverterapp"
    )
}

fn backend_is_libredwg(value: &str) -> bool {
    matches!(normalize_backend(value).as_str(), "libredwg" | "dwg2dxf")
}

fn backend_is_freecad(value: &str) -> bool {
    matches!(normalize_backend(value).as_str(), "freecad" | "freecadcmd")
}

fn normalize_backend(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn log_backend_status(backend: &DwgBackend) {
    if let Some(path) = &backend.command {
        log_dwg(format!(
            "DWG backend {} path={} exists={} executable={} automatic={} source={}",
            backend.name,
            path.display(),
            path.exists(),
            is_executable(path),
            backend.automatic,
            backend.source.as_deref().unwrap_or("unknown")
        ));
    } else {
        log_dwg(format!(
            "DWG backend {} not found automatic={} source={}",
            backend.name,
            backend.automatic,
            backend.source.as_deref().unwrap_or("unknown")
        ));
    }
}

fn log_command_output(backend: &DwgBackend, stdout: &[u8], stderr: &[u8]) {
    log_dwg(format!(
        "{} stdout: {}",
        backend.name,
        String::from_utf8_lossy(stdout).trim()
    ));
    log_dwg(format!(
        "{} stderr: {}",
        backend.name,
        String::from_utf8_lossy(stderr).trim()
    ));
}

fn log_dwg(message: impl AsRef<str>) {
    let line = format!("[lixcad-dwg] {}\n", message.as_ref());
    eprint!("{line}");
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/nodalix-cad.log")
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(line.as_bytes())
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_oda_config_alias() {
        let config = parse_config_data(
            r#"
            [dwg_import]
            backend = "oda"
            converter_path = "/usr/bin/oda-file-converter"
            "#,
        );
        assert_eq!(config.backend.as_deref(), Some("oda"));
        assert_eq!(
            config.converter_path.as_deref(),
            Some(Path::new("/usr/bin/oda-file-converter"))
        );
        assert!(backend_is_oda(config.backend.as_deref().unwrap()));
    }

    #[test]
    fn exposes_expected_settings_paths() {
        let paths = settings_paths();
        assert!(paths
            .iter()
            .any(|path| path.ends_with("lixcad/settings.toml")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with(".config/lixcad/settings.toml")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with(".config/nodalix-cad/settings.toml")));
    }
}
