use serde::Serialize;
use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub created: Option<u64>,
    pub icon_kind: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SpecialDirs {
    pub home: String,
    pub desktop: Option<String>,
    pub downloads: Option<String>,
    pub documents: Option<String>,
    pub pictures: Option<String>,
    pub videos: Option<String>,
    pub music: Option<String>,
    pub data: Option<String>,
    pub trash: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PathProperties {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub kind_label: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub size: u64,
    pub size_display: String,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub accessed: Option<u64>,
    pub permissions: String,
    pub readonly: bool,
    pub owner: Option<u32>,
    pub group: Option<u32>,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
    pub item_count: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct OpenWithApp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct StartupBundle {
    pub home: String,
    pub initial_path: Option<String>,
    pub platform: crate::platform::PlatformInfo,
    pub sidebar_items: Vec<crate::sidebar::SidebarItem>,
    pub localsend_available: bool,
    pub folder_customizations:
        std::collections::HashMap<String, crate::folder_customization::FolderStyle>,
    pub special_dirs: SpecialDirs,
}

fn initial_launch_path() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .filter(|path| Path::new(path).is_dir())
}

fn virtual_dir_entry(path: &Path, name: String, size: u64, icon_kind: &str) -> FileEntry {
    FileEntry {
        name,
        path: path_to_string_fast(path),
        is_dir: true,
        size,
        modified: None,
        created: None,
        icon_kind: Some(icon_kind.into()),
    }
}

fn unescape_mount_path(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

fn path_is_inside_mount(path: &Path, mountpoint: &Path) -> bool {
    path == mountpoint || path.starts_with(mountpoint)
}

fn is_slow_mount_fs(fs_type: &str, source: &str) -> bool {
    let fs_type = fs_type.to_ascii_lowercase();
    let source = source.to_ascii_lowercase();
    fs_type.starts_with("fuse")
        || fs_type.contains("rclone")
        || fs_type == "sshfs"
        || fs_type == "nfs"
        || fs_type == "nfs4"
        || fs_type == "cifs"
        || fs_type == "smb3"
        || source.contains("rclone")
        || source.contains("gvfs")
}

fn path_uses_slow_metadata(path: &Path) -> bool {
    let Ok(mounts) = fs::read_to_string("/proc/self/mountinfo") else {
        return false;
    };
    let mut best_mount_len = 0usize;
    let mut slow = false;

    for line in mounts.lines() {
        let Some((before, after)) = line.split_once(" - ") else {
            continue;
        };
        let before_fields: Vec<&str> = before.split_whitespace().collect();
        let after_fields: Vec<&str> = after.split_whitespace().collect();
        if before_fields.len() < 5 || after_fields.len() < 2 {
            continue;
        }

        let mountpoint = PathBuf::from(unescape_mount_path(before_fields[4]));
        if !path_is_inside_mount(path, &mountpoint) {
            continue;
        }
        let mount_len = mountpoint.to_string_lossy().len();
        if mount_len < best_mount_len {
            continue;
        }
        best_mount_len = mount_len;
        slow = is_slow_mount_fs(after_fields[0], after_fields[1]);
    }

    slow
}

#[derive(Clone, Copy)]
enum ClipOp {
    Copy,
    Cut,
}

struct Clipboard {
    op: ClipOp,
    paths: Vec<String>,
}

static SPECIAL_DIRS: OnceLock<Result<SpecialDirs, String>> = OnceLock::new();
static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);
static SEARCH_GENERATION: AtomicU64 = AtomicU64::new(0);

fn err(msg: impl Into<String>) -> String {
    msg.into()
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn image_mime(path: &Path) -> Option<&'static str> {
    match extension_lower(path).as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

fn write_u16(file: &mut fs::File, value: u16) -> io::Result<()> {
    file.write_all(&value.to_le_bytes())
}

fn write_u32(file: &mut fs::File, value: u32) -> io::Result<()> {
    file.write_all(&value.to_le_bytes())
}

fn write_stored_zip(path: &Path, entries: &[(&str, &str)]) -> io::Result<()> {
    struct CentralEntry {
        name: Vec<u8>,
        crc: u32,
        len: u32,
        offset: u32,
    }

    let mut file = fs::File::create(path)?;
    let mut central = Vec::with_capacity(entries.len());

    for (name, content) in entries {
        let name_bytes = name.as_bytes();
        let content_bytes = content.as_bytes();
        let offset = file.stream_position()? as u32;
        let crc = crc32(content_bytes);
        let len = content_bytes.len() as u32;

        write_u32(&mut file, 0x0403_4b50)?;
        write_u16(&mut file, 20)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u32(&mut file, crc)?;
        write_u32(&mut file, len)?;
        write_u32(&mut file, len)?;
        write_u16(&mut file, name_bytes.len() as u16)?;
        write_u16(&mut file, 0)?;
        file.write_all(name_bytes)?;
        file.write_all(content_bytes)?;

        central.push(CentralEntry {
            name: name_bytes.to_vec(),
            crc,
            len,
            offset,
        });
    }

    let central_start = file.stream_position()? as u32;
    for entry in &central {
        write_u32(&mut file, 0x0201_4b50)?;
        write_u16(&mut file, 20)?;
        write_u16(&mut file, 20)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u32(&mut file, entry.crc)?;
        write_u32(&mut file, entry.len)?;
        write_u32(&mut file, entry.len)?;
        write_u16(&mut file, entry.name.len() as u16)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u32(&mut file, 0)?;
        write_u32(&mut file, entry.offset)?;
        file.write_all(&entry.name)?;
    }

    let central_size = file.stream_position()? as u32 - central_start;
    write_u32(&mut file, 0x0605_4b50)?;
    write_u16(&mut file, 0)?;
    write_u16(&mut file, 0)?;
    write_u16(&mut file, central.len() as u16)?;
    write_u16(&mut file, central.len() as u16)?;
    write_u32(&mut file, central_size)?;
    write_u32(&mut file, central_start)?;
    write_u16(&mut file, 0)?;
    Ok(())
}

fn write_document_template(path: &Path) -> io::Result<bool> {
    match extension_lower(path).as_str() {
        "docx" => {
            write_stored_zip(
                path,
                &[
                    (
                        "[Content_Types].xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
                    ),
                    (
                        "_rels/.rels",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
                    ),
                    (
                        "word/document.xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p/><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#,
                    ),
                ],
            )?;
            Ok(true)
        }
        "xlsx" => {
            write_stored_zip(
                path,
                &[
                    (
                        "[Content_Types].xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
                    ),
                    (
                        "_rels/.rels",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                    ),
                    (
                        "xl/_rels/workbook.xml.rels",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
                    ),
                    (
                        "xl/workbook.xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Hoja1" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                    ),
                    (
                        "xl/worksheets/sheet1.xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#,
                    ),
                ],
            )?;
            Ok(true)
        }
        "pptx" => {
            write_stored_zip(
                path,
                &[
                    (
                        "[Content_Types].xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/></Types>"#,
                    ),
                    (
                        "_rels/.rels",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
                    ),
                    (
                        "ppt/presentation.xml",
                        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst/><p:sldIdLst/><p:sldSz cx="9144000" cy="6858000" type="screen4x3"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#,
                    ),
                ],
            )?;
            Ok(true)
        }
        "odt" | "ods" | "odp" => {
            let mimetype = match extension_lower(path).as_str() {
                "ods" => "application/vnd.oasis.opendocument.spreadsheet",
                "odp" => "application/vnd.oasis.opendocument.presentation",
                _ => "application/vnd.oasis.opendocument.text",
            };
            write_stored_zip(
                path,
                &[
                    ("mimetype", mimetype),
                    (
                        "META-INF/manifest.xml",
                        r#"<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/></manifest:manifest>"#,
                    ),
                    (
                        "content.xml",
                        r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" office:version="1.2"><office:body><office:text/></office:body></office:document-content>"#,
                    ),
                ],
            )?;
            Ok(true)
        }
        "pdf" => {
            fs::write(
                path,
                b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 0/Kids[]>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n",
            )?;
            Ok(true)
        }
        "rtf" => {
            fs::write(path, b"{\\rtf1\\ansi\\deff0\n}\n")?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn create_typed_document(path: &Path) -> io::Result<()> {
    if !write_document_template(path)? {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
    }
    Ok(())
}

fn file_type_details(path: &Path, is_dir: bool, is_symlink: bool) -> (String, Option<String>) {
    if is_dir {
        return ("Carpeta".into(), Some("inode/directory".into()));
    }
    if is_symlink {
        return ("Enlace simbólico".into(), None);
    }

    let ext = extension_lower(path);
    let detail = match ext.as_str() {
        "txt" => ("Documento de texto", "text/plain"),
        "md" | "markdown" => ("Documento Markdown", "text/markdown"),
        "rtf" => ("Documento RTF", "application/rtf"),
        "pdf" => ("Documento PDF", "application/pdf"),
        "doc" => ("Documento Word antiguo", "application/msword"),
        "docx" => (
            "Documento Word",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        "odt" => (
            "Documento OpenDocument",
            "application/vnd.oasis.opendocument.text",
        ),
        "xls" => ("Hoja de cálculo Excel antigua", "application/vnd.ms-excel"),
        "xlsx" => (
            "Hoja de cálculo Excel",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        "ods" => (
            "Hoja de cálculo OpenDocument",
            "application/vnd.oasis.opendocument.spreadsheet",
        ),
        "csv" => ("Tabla CSV", "text/csv"),
        "ppt" => (
            "Presentación PowerPoint antigua",
            "application/vnd.ms-powerpoint",
        ),
        "pptx" => (
            "Presentación PowerPoint",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        "odp" => (
            "Presentación OpenDocument",
            "application/vnd.oasis.opendocument.presentation",
        ),
        "dwg" => ("Dibujo AutoCAD DWG", "image/vnd.dwg"),
        "dxf" => ("Intercambio CAD DXF", "image/vnd.dxf"),
        "nodcad" | "lixcad" => ("Documento Lix CAD", "application/x-lixcad"),
        "step" | "stp" => ("Modelo STEP", "model/step"),
        "iges" | "igs" => ("Modelo IGES", "model/iges"),
        "stl" => ("Malla STL", "model/stl"),
        "obj" => ("Malla OBJ", "model/obj"),
        "ply" => ("Malla PLY", "model/ply"),
        "jpg" | "jpeg" => ("Imagen JPEG", "image/jpeg"),
        "png" => ("Imagen PNG", "image/png"),
        "gif" => ("Imagen GIF", "image/gif"),
        "webp" => ("Imagen WebP", "image/webp"),
        "svg" => ("Imagen SVG", "image/svg+xml"),
        "tif" | "tiff" => ("Imagen TIFF", "image/tiff"),
        "heic" => ("Imagen HEIC", "image/heic"),
        "heif" => ("Imagen HEIF", "image/heif"),
        "mp3" => ("Audio MP3", "audio/mpeg"),
        "flac" => ("Audio FLAC", "audio/flac"),
        "wav" => ("Audio WAV", "audio/wav"),
        "ogg" => ("Audio Ogg", "audio/ogg"),
        "mp4" => ("Vídeo MP4", "video/mp4"),
        "mkv" => ("Vídeo Matroska", "video/x-matroska"),
        "webm" => ("Vídeo WebM", "video/webm"),
        "zip" => ("Archivo ZIP", "application/zip"),
        "tar" => ("Archivo TAR", "application/x-tar"),
        "gz" => ("Archivo GZip", "application/gzip"),
        "7z" => ("Archivo 7-Zip", "application/x-7z-compressed"),
        "rar" => ("Archivo RAR", "application/vnd.rar"),
        "py" => ("Script Python", "text/x-python"),
        "c" => ("Código C", "text/x-c"),
        "h" => ("Cabecera C/C++", "text/x-c"),
        "cpp" | "hpp" => ("Código C++", "text/x-c++"),
        "rs" => ("Código Rust", "text/rust"),
        "js" | "jsx" => ("Código JavaScript", "text/javascript"),
        "ts" | "tsx" => ("Código TypeScript", "text/typescript"),
        "json" => ("Documento JSON", "application/json"),
        "html" => ("Documento HTML", "text/html"),
        "css" => ("Hoja de estilos CSS", "text/css"),
        "sh" | "bash" | "zsh" => ("Script de shell", "text/x-shellscript"),
        "" => ("Archivo sin extensión", "application/octet-stream"),
        _ => ("Archivo", "application/octet-stream"),
    };
    (detail.0.into(), Some(detail.1.into()))
}

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "No se ha definido HOME".to_string())
}

fn expand_user_dir(value: &str, home: &Path) -> PathBuf {
    PathBuf::from(value.replace("$HOME", &home.to_string_lossy()))
}

fn read_user_dirs_entry(home: &Path, key: &str) -> Option<PathBuf> {
    let config = home.join(".config/user-dirs.dirs");
    let content = fs::read_to_string(config).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        if k.trim() != key {
            continue;
        }
        let raw = v.trim().trim_matches('"');
        return Some(expand_user_dir(raw, home));
    }
    None
}

fn resolve_user_dir(
    home: &Path,
    xdg_env: &str,
    user_dirs_key: &str,
    fallback_name: &str,
) -> Option<String> {
    if let Ok(raw) = std::env::var(xdg_env) {
        let path = expand_user_dir(&raw, home);
        if path.is_dir() {
            return Some(path_to_string_fast(&path));
        }
    }
    if let Some(path) = read_user_dirs_entry(home, user_dirs_key) {
        if path.is_dir() {
            return Some(path_to_string_fast(&path));
        }
    }
    let fallback = home.join(fallback_name);
    if fallback.is_dir() {
        return Some(path_to_string_fast(&fallback));
    }
    None
}

fn path_to_string_fast(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    if bytes < 1024 * 1024 * 1024 {
        return format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0);
    }
    format!("{:.1} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
}

fn compute_special_dirs() -> Result<SpecialDirs, String> {
    crate::platform::debug_log("compute_special_dirs");
    let home = home_dir()?;
    let home_str = path_to_string_fast(&home);
    let trash = home.join(".local/share/Trash/files");
    let trash = match fs::create_dir_all(&trash) {
        Ok(_) => Some(path_to_string_fast(&trash)),
        Err(_) => None,
    };
    Ok(SpecialDirs {
        home: home_str,
        desktop: resolve_user_dir(&home, "XDG_DESKTOP_DIR", "XDG_DESKTOP_DIR", "Desktop"),
        downloads: resolve_user_dir(&home, "XDG_DOWNLOAD_DIR", "XDG_DOWNLOAD_DIR", "Downloads"),
        documents: resolve_user_dir(&home, "XDG_DOCUMENTS_DIR", "XDG_DOCUMENTS_DIR", "Documents"),
        pictures: resolve_user_dir(&home, "XDG_PICTURES_DIR", "XDG_PICTURES_DIR", "Pictures"),
        videos: resolve_user_dir(&home, "XDG_VIDEOS_DIR", "XDG_VIDEOS_DIR", "Videos"),
        music: resolve_user_dir(&home, "XDG_MUSIC_DIR", "XDG_MUSIC_DIR", "Music"),
        data: if Path::new("/data").is_dir() {
            Some("/data".into())
        } else {
            None
        },
        trash,
    })
}

pub fn get_special_dirs_cached() -> Result<SpecialDirs, String> {
    SPECIAL_DIRS.get_or_init(compute_special_dirs).clone()
}

#[tauri::command]
pub fn get_special_dirs() -> Result<SpecialDirs, String> {
    get_special_dirs_cached()
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    crate::platform::debug_log(&format!("list_directory {path}"));
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(err(format!("No es una carpeta: {path}")));
    }

    let read_dir = fs::read_dir(&dir).map_err(|e| format!("No se puede leer {path}: {e}"))?;
    let mut entries = Vec::new();
    let lightweight = path_uses_slow_metadata(&dir);
    if lightweight {
        crate::platform::debug_log(&format!("list_directory_lightweight {path}"));
    }

    for item in read_dir {
        let item = match item {
            Ok(i) => i,
            Err(_) => continue,
        };
        let file_name = item.file_name().to_string_lossy().into_owned();
        if file_name == "." || file_name == ".." {
            continue;
        }

        let file_type = match item.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let is_dir = file_type.is_dir();
        let file_path = item.path();
        let full_path = path_to_string_fast(&file_path);

        let metadata = if lightweight {
            None
        } else {
            item.metadata().ok()
        };
        let size = if is_dir {
            0
        } else {
            metadata.as_ref().map(|m| m.len()).unwrap_or(0)
        };
        let modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let created = metadata
            .as_ref()
            .and_then(|m| m.created().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .or(modified);

        entries.push(FileEntry {
            name: file_name,
            path: full_path,
            is_dir,
            size,
            modified,
            created,
            icon_kind: None,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

#[tauri::command]
pub fn search_directory(
    path: String,
    query: String,
    show_hidden: bool,
    extensions: Vec<String>,
) -> Result<Vec<FileEntry>, String> {
    const MAX_RESULTS: usize = 80;
    const MAX_VISITED_DIRS: usize = 260;
    const MAX_SEARCH_TIME: Duration = Duration::from_millis(900);

    let needle = query.trim().to_ascii_lowercase();
    let extensions: Vec<String> = extensions
        .into_iter()
        .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect();
    if needle.len() < 2 && extensions.is_empty() {
        return Ok(Vec::new());
    }
    let generation = SEARCH_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;

    crate::platform::debug_log(&format!("search_directory {path} {needle}"));
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(err(format!("No es una carpeta: {path}")));
    }

    let lightweight = path_uses_slow_metadata(&root);
    let started = Instant::now();
    let mut queue = VecDeque::from([root]);
    let mut entries = Vec::new();
    let mut visited_dirs = 0usize;

    while let Some(dir) = queue.pop_front() {
        if SEARCH_GENERATION.load(Ordering::Relaxed) != generation {
            return Ok(Vec::new());
        }
        if started.elapsed() >= MAX_SEARCH_TIME {
            break;
        }
        visited_dirs += 1;
        if visited_dirs > MAX_VISITED_DIRS || entries.len() >= MAX_RESULTS {
            break;
        }

        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for item in read_dir.flatten() {
            let file_name = item.file_name().to_string_lossy().into_owned();
            if file_name == "." || file_name == ".." {
                continue;
            }
            if entries.len() >= MAX_RESULTS || started.elapsed() >= MAX_SEARCH_TIME {
                break;
            }
            if !show_hidden && file_name.starts_with('.') {
                continue;
            }
            if is_search_pruned_dir(&file_name) {
                continue;
            }

            let Ok(file_type) = item.file_type() else {
                continue;
            };
            let is_dir = file_type.is_dir();
            let file_path = item.path();
            if is_dir {
                queue.push_back(file_path.clone());
            }

            if !needle.is_empty() && !file_name.to_ascii_lowercase().contains(&needle) {
                continue;
            }
            if !extensions.is_empty() && !entry_matches_extensions(&file_path, &extensions) {
                continue;
            }

            let metadata = if lightweight {
                None
            } else {
                item.metadata().ok()
            };
            let size = if is_dir {
                0
            } else {
                metadata.as_ref().map(|m| m.len()).unwrap_or(0)
            };
            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let created = metadata
                .as_ref()
                .and_then(|m| m.created().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .or(modified);

            entries.push(FileEntry {
                name: file_name,
                path: path_to_string_fast(&file_path),
                is_dir,
                size,
                modified,
                created,
                icon_kind: None,
            });

            if entries.len() >= MAX_RESULTS {
                break;
            }
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
    });

    Ok(entries)
}

fn entry_matches_extensions(path: &Path, extensions: &[String]) -> bool {
    let ext = extension_lower(path);
    extensions.iter().any(|candidate| candidate == &ext)
}

fn is_search_pruned_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".cache"
            | ".cargo"
            | ".npm"
            | ".pnpm-store"
            | ".rustup"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
    )
}

#[tauri::command]
pub fn list_disks() -> Result<Vec<FileEntry>, String> {
    #[derive(Default)]
    struct DiskInfo {
        name: String,
        label: String,
        model: String,
        size: u64,
        mountpoints: Vec<String>,
    }

    fn read_lsblk_field(line: &str, key: &str) -> String {
        let needle = format!("{key}=\"");
        let Some(start) = line.find(&needle).map(|idx| idx + needle.len()) else {
            return String::new();
        };
        let rest = &line[start..];
        let Some(end) = rest.find('"') else {
            return String::new();
        };
        rest[..end].to_string()
    }

    fn useful_mountpoint(path: &str) -> bool {
        path == "/"
            || path == "/home"
            || path == "/data"
            || path.starts_with("/media/")
            || path.starts_with("/mnt/")
            || path.starts_with("/run/media/")
    }

    fn mount_priority(path: &str) -> u8 {
        if path == "/data" {
            0
        } else if path == "/home" {
            1
        } else if path.starts_with("/run/media/") || path.starts_with("/media/") {
            2
        } else if path.starts_with("/mnt/") {
            3
        } else if path == "/" {
            4
        } else {
            9
        }
    }

    fn best_mountpoint(mounts: &[String]) -> Option<String> {
        mounts
            .iter()
            .filter(|mount| useful_mountpoint(mount) && Path::new(mount.as_str()).is_dir())
            .min_by_key(|mount| mount_priority(mount))
            .cloned()
    }

    let output = Command::new("lsblk")
        .args(["-P", "-b", "-o", "NAME,LABEL,MODEL,SIZE,MOUNTPOINT,TYPE"])
        .output()
        .map_err(|e| format!("No se pudieron listar los discos con lsblk: {e}"))?;
    if !output.status.success() {
        return Err(err("No se pudieron listar los discos"));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::<DiskInfo>::new();
    let mut current: Option<DiskInfo> = None;
    for line in raw.lines() {
        let item_type = read_lsblk_field(line, "TYPE");
        let name = read_lsblk_field(line, "NAME");
        let label = read_lsblk_field(line, "LABEL");
        let model = read_lsblk_field(line, "MODEL").trim().to_string();
        let size = read_lsblk_field(line, "SIZE").parse::<u64>().unwrap_or(0);
        let mountpoint = read_lsblk_field(line, "MOUNTPOINT");

        if item_type == "disk" {
            if name.starts_with("zram") || mountpoint == "[SWAP]" {
                current = None;
                continue;
            }
            if let Some(disk) = current.take() {
                disks.push(disk);
            }
            current = Some(DiskInfo {
                name,
                label,
                model,
                size,
                mountpoints: if useful_mountpoint(&mountpoint) {
                    vec![mountpoint]
                } else {
                    Vec::new()
                },
            });
        } else {
            let Some(disk) = current.as_mut() else {
                continue;
            };
            if useful_mountpoint(&mountpoint) {
                disk.mountpoints.push(mountpoint);
            }
            if disk.label.is_empty() && !label.is_empty() {
                disk.label = label;
            }
        }
    }
    if let Some(disk) = current.take() {
        disks.push(disk);
    }

    let mut entries = Vec::new();
    let mut used_mountpoints = std::collections::HashSet::<String>::new();
    let home_path = home_dir().ok();
    for (index, disk) in disks.into_iter().enumerate() {
        let mut mountpoint = best_mountpoint(&disk.mountpoints);
        if mountpoint.is_none() && index == 0 {
            mountpoint = home_path.as_ref().map(|path| path_to_string_fast(path));
        }
        if mountpoint.is_none() && index > 0 && Path::new("/data").is_dir() {
            mountpoint = Some("/data".into());
        }
        let Some(mountpoint) = mountpoint else {
            continue;
        };
        if !used_mountpoints.insert(mountpoint.clone()) {
            continue;
        }
        let path = PathBuf::from(&mountpoint);
        if !path.is_dir() {
            continue;
        }
        let base = if !disk.label.is_empty() {
            disk.label
        } else if !disk.model.is_empty() {
            disk.model
        } else {
            disk.name
        };
        let label = if disk.size > 0 {
            format!("{base} · {}", format_size(disk.size))
        } else {
            base
        };
        entries.push(virtual_dir_entry(&path, label, disk.size, "disk"));
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

#[tauri::command]
pub fn list_network_locations() -> Result<Vec<FileEntry>, String> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/run/user").join(std::process::id().to_string()));
    let gvfs = runtime.join("gvfs");
    if !gvfs.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for item in
        fs::read_dir(&gvfs).map_err(|e| format!("No se pueden leer los montajes de red: {e}"))?
    {
        let item = match item {
            Ok(item) => item,
            Err(_) => continue,
        };
        let path = item.path();
        if !path.is_dir() {
            continue;
        }
        let name = item
            .file_name()
            .to_string_lossy()
            .replace("smb-share:", "SMB ");
        entries.push(virtual_dir_entry(&path, name, 0, "network"));
    }
    Ok(entries)
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err(err("No hay nada que copiar"));
    }
    let commands: [(&str, &[&str]); 3] = [
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];
    for (program, args) in commands {
        let mut child = match Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => continue,
        };
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            if stdin.write_all(text.as_bytes()).is_err() {
                continue;
            }
        }
        if child.wait().map(|status| status.success()).unwrap_or(false) {
            return Ok(());
        }
    }
    Err(err("No hay ningún comando de portapapeles disponible"))
}

fn encode_uri_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if keep {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[tauri::command]
pub fn connect_network_location(
    protocol: String,
    host: String,
    share: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), String> {
    let protocol = protocol.trim().to_lowercase();
    if protocol != "smb" && protocol != "ftp" && protocol != "sftp" {
        return Err(err("Protocolo de red no compatible"));
    }
    let host = host.trim();
    if host.is_empty() || host.contains('/') || host.contains('\\') {
        return Err(err("Introduce un nombre de host o dirección válidos"));
    }
    let share = share.trim().trim_matches('/');
    let user = username.unwrap_or_default();
    let password = password.unwrap_or_default();

    let authority = if user.trim().is_empty() {
        host.to_string()
    } else if password.is_empty() {
        format!("{}@{host}", encode_uri_component(user.trim()))
    } else {
        format!(
            "{}:{}@{host}",
            encode_uri_component(user.trim()),
            encode_uri_component(&password)
        )
    };
    let uri = if share.is_empty() {
        format!("{protocol}://{authority}/")
    } else {
        format!("{protocol}://{authority}/{}", encode_uri_component(share))
    };

    let output = Command::new("gio")
        .args(["mount", &uri])
        .output()
        .map_err(|e| format!("No se pudo iniciar gio mount: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(err("No se pudo conectar a la ubicación de red"))
    } else {
        Err(stderr)
    }
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(err(format!("La ruta no existe: {path}")));
    }
    Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("No se pudo iniciar xdg-open: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_with(path: String, app_id: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(err(format!("La ruta no existe: {path}")));
    }
    if app_id == "xdg-default" {
        return open_path(path);
    }
    Command::new("gtk-launch")
        .arg(&app_id)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("gtk-launch falló: {e}. Prueba xdg-default."))?;
    Ok(())
}

#[tauri::command]
pub fn get_open_with_apps(path: String) -> Result<Vec<OpenWithApp>, String> {
    let _ = path;
    Ok(vec![
        OpenWithApp {
            id: "xdg-default".into(),
            name: "Aplicación predeterminada".into(),
        },
        OpenWithApp {
            id: "org.gnome.TextEditor".into(),
            name: "Editor de texto".into(),
        },
        OpenWithApp {
            id: "code".into(),
            name: "Visual Studio Code".into(),
        },
        OpenWithApp {
            id: "org.gnome.eog".into(),
            name: "Visor de imágenes".into(),
        },
        OpenWithApp {
            id: "vlc".into(),
            name: "VLC".into(),
        },
    ])
}

#[tauri::command]
pub fn create_folder(parent: String, name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(err("El nombre de la carpeta no puede estar vacío"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(err(
            "El nombre de la carpeta no puede contener separadores de ruta",
        ));
    }
    let parent_path = PathBuf::from(&parent);
    if !parent_path.is_dir() {
        return Err(err(format!(
            "La ubicación padre no es una carpeta: {parent}"
        )));
    }
    let new_path = parent_path.join(trimmed);
    if new_path.exists() {
        return Err(err(format!("Ya existe: {}", new_path.display())));
    }
    fs::create_dir(&new_path).map_err(|e| e.to_string())?;
    Ok(path_to_string_fast(&new_path))
}

#[tauri::command]
pub fn create_document(parent: String, name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(err("El nombre del documento no puede estar vacío"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(err(
            "El nombre del documento no puede contener separadores de ruta",
        ));
    }
    let parent_path = PathBuf::from(&parent);
    if !parent_path.is_dir() {
        return Err(err(format!(
            "La ubicación padre no es una carpeta: {parent}"
        )));
    }
    let new_path = parent_path.join(trimmed);
    if new_path.exists() {
        return Err(err(format!("Ya existe: {}", new_path.display())));
    }
    create_typed_document(&new_path).map_err(|e| e.to_string())?;
    Ok(path_to_string_fast(&new_path))
}

#[tauri::command]
pub fn rename_path(old_path: String, new_name: String) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(err("El nuevo nombre no puede estar vacío"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(err("El nuevo nombre no puede contener separadores de ruta"));
    }
    let source = PathBuf::from(&old_path);
    if !source.exists() {
        return Err(err(format!("La ruta no existe: {old_path}")));
    }
    let parent = source
        .parent()
        .ok_or_else(|| err("No se puede resolver la carpeta padre"))?;
    let dest = parent.join(trimmed);
    if dest.exists() {
        return Err(err(format!("Ya existe: {}", dest.display())));
    }
    let should_materialize_template = fs::metadata(&source)
        .map(|metadata| metadata.is_file() && metadata.len() == 0)
        .unwrap_or(false);
    fs::rename(&source, &dest).map_err(|e| e.to_string())?;
    if should_materialize_template {
        write_document_template(&dest).map_err(|e| e.to_string())?;
    }
    Ok(path_to_string_fast(&dest))
}

#[tauri::command]
pub fn trash_path(path: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(err(format!("La ruta no existe: {path}")));
    }
    match Command::new("gio").args(["trash", &path]).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(err(format!("gio trash falló (salida {status})"))),
        Err(e) => Err(err(format!(
            "gio trash no está disponible ({e}). Instala glib2 (gio)."
        ))),
    }
}

#[tauri::command]
pub fn trash_paths(paths: Vec<String>) -> Result<(), String> {
    for path in paths {
        trash_path(path)?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_paths_permanently(paths: Vec<String>) -> Result<(), String> {
    for path in paths {
        let target = PathBuf::from(&path);
        if !target.exists() {
            continue;
        }
        let meta = fs::symlink_metadata(&target)
            .map_err(|e| format!("No se pudo inspeccionar {path}: {e}"))?;
        if meta.is_dir() {
            fs::remove_dir_all(&target).map_err(|e| format!("No se pudo eliminar {path}: {e}"))?;
        } else {
            fs::remove_file(&target).map_err(|e| format!("No se pudo eliminar {path}: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn copy_paths(paths: Vec<String>) -> Result<(), String> {
    let mut clip = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    *clip = Some(Clipboard {
        op: ClipOp::Copy,
        paths,
    });
    Ok(())
}

#[tauri::command]
pub fn cut_paths(paths: Vec<String>) -> Result<(), String> {
    let mut clip = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    *clip = Some(Clipboard {
        op: ClipOp::Cut,
        paths,
    });
    Ok(())
}

#[tauri::command]
pub fn clipboard_has_content() -> bool {
    CLIPBOARD.lock().map(|c| c.is_some()).unwrap_or(false)
}

fn copy_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            copy_recursive(&entry.path(), &dst.join(name))?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

#[tauri::command]
pub fn paste_into(target_dir: String) -> Result<(), String> {
    let target = PathBuf::from(&target_dir);
    if !target.is_dir() {
        return Err(err("El destino de pegado debe ser una carpeta"));
    }
    let guard = CLIPBOARD.lock().map_err(|_| err("clipboard lock"))?;
    let clip = guard.as_ref().ok_or_else(|| err("No hay nada que pegar"))?;
    let paths = clip.paths.clone();
    let op = match clip.op {
        ClipOp::Copy => ClipOp::Copy,
        ClipOp::Cut => ClipOp::Cut,
    };

    for src_str in &paths {
        let src = PathBuf::from(src_str);
        if !src.exists() {
            continue;
        }
        let name = src
            .file_name()
            .ok_or_else(|| err("Ruta de origen no válida"))?
            .to_owned();
        let mut dest = target.join(&name);
        if dest.exists() {
            dest = unique_path(&dest);
        }
        match op {
            ClipOp::Copy => copy_recursive(&src, &dest).map_err(|e| e.to_string())?,
            ClipOp::Cut => fs::rename(&src, &dest).map_err(|e| e.to_string())?,
        }
    }

    if matches!(op, ClipOp::Cut) {
        *CLIPBOARD.lock().map_err(|_| err("clipboard lock"))? = None;
    }
    Ok(())
}

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "copia".into());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(Path::new("."));
    for i in 1..1000 {
        let candidate = parent.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

#[tauri::command]
pub fn move_to(paths: Vec<String>, target_dir: String) -> Result<(), String> {
    let target = PathBuf::from(&target_dir);
    if !target.is_dir() {
        return Err(err("El destino debe ser una carpeta"));
    }
    let target_canon = target
        .canonicalize()
        .map_err(|e| format!("No se puede resolver la carpeta de destino: {e}"))?;

    let mut moves = Vec::new();
    for src_str in &paths {
        let src = PathBuf::from(src_str);
        if !src.exists() {
            return Err(err(format!("La ruta no existe: {src_str}")));
        }
        let src_canon = src
            .canonicalize()
            .map_err(|e| format!("No se puede resolver la ruta de origen {src_str}: {e}"))?;
        if src_canon == target_canon {
            return Err(err("No se puede mover un elemento dentro de sí mismo"));
        }
        let meta = fs::symlink_metadata(&src)
            .map_err(|e| format!("No se pudo inspeccionar {src_str}: {e}"))?;
        if meta.is_dir() && target_canon.starts_with(&src_canon) {
            return Err(err(
                "No se puede mover una carpeta dentro de sí misma ni de uno de sus descendientes",
            ));
        }
        let name = src.file_name().ok_or_else(|| err("Ruta no válida"))?;
        let dest = target.join(name);
        if dest.exists() {
            return Err(err(format!("Ya existe: {}", dest.display())));
        }
        moves.push((src, dest));
    }

    for (src, dest) in moves {
        fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ts_secs(
    path: &Path,
    field: fn(&fs::Metadata) -> io::Result<std::time::SystemTime>,
) -> Option<u64> {
    let meta = fs::symlink_metadata(path).ok()?;
    field(&meta)
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn count_dir_items(path: &Path) -> Option<u64> {
    const MAX: u64 = 5000;
    let read = fs::read_dir(path).ok()?;
    let mut count = 0u64;
    for entry in read {
        if entry.is_ok() {
            count += 1;
            if count >= MAX {
                return Some(MAX);
            }
        }
    }
    Some(count)
}

fn thumbnail_cache_path(path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    let stamp = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::env::temp_dir()
        .join("nodalix-files-thumbnails")
        .join(format!("{:x}-{stamp}.png", hasher.finish()))
}

fn thumbnail_fail_path(path: &Path) -> PathBuf {
    thumbnail_cache_path(path).with_extension("failed")
}

fn mark_thumbnail_failed(path: &Path) {
    let failed = thumbnail_fail_path(path);
    if let Some(parent) = failed.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(failed, b"failed");
}

fn quiet_status(command: &mut Command) -> Result<std::process::ExitStatus, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pdf_thumbnail(path: String) -> Result<Option<String>, String> {
    let source = PathBuf::from(&path);
    if extension_lower(&source) != "pdf" || !source.is_file() {
        return Ok(None);
    }

    let output = thumbnail_cache_path(&source);
    if output.exists() {
        return Ok(Some(path_to_string_fast(&output)));
    }
    if thumbnail_fail_path(&source).exists() {
        return Ok(None);
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let output_base = output.with_extension("");
    if which("pdftoppm") {
        let status = quiet_status(
            Command::new("pdftoppm")
                .args(["-f", "1", "-singlefile", "-png", "-scale-to", "384"])
                .arg(&source)
                .arg(&output_base),
        )
        .map_err(|e| format!("No se pudo iniciar pdftoppm: {e}"))?;
        if status.success() && output.exists() {
            return Ok(Some(path_to_string_fast(&output)));
        }

        let status = quiet_status(
            Command::new("pdftoppm")
                .args(["-f", "1", "-l", "1", "-singlefile", "-png", "-r", "96"])
                .arg(&source)
                .arg(&output_base),
        )
        .map_err(|e| format!("No se pudo iniciar pdftoppm: {e}"))?;
        if status.success() && output.exists() {
            return Ok(Some(path_to_string_fast(&output)));
        }
    }

    if which("mutool") {
        let status = quiet_status(
            Command::new("mutool")
                .args(["draw", "-q", "-o"])
                .arg(&output)
                .args(["-w", "384", "-h", "384"])
                .arg(&source)
                .arg("1"),
        )
        .map_err(|e| format!("No se pudo iniciar mutool: {e}"))?;
        if status.success() && output.exists() {
            return Ok(Some(path_to_string_fast(&output)));
        }
    }

    if which("magick") {
        let first_page = format!("{}[0]", source.display());
        let status = quiet_status(
            Command::new("magick")
                .arg(first_page)
                .args([
                    "-thumbnail",
                    "384x384",
                    "-background",
                    "white",
                    "-alpha",
                    "remove",
                    "-alpha",
                    "off",
                ])
                .arg(&output),
        )
        .map_err(|e| format!("No se pudo iniciar magick: {e}"))?;
        if status.success() && output.exists() {
            return Ok(Some(path_to_string_fast(&output)));
        }
    }

    if which("convert") {
        let first_page = format!("{}[0]", source.display());
        let status = quiet_status(
            Command::new("convert")
                .arg(first_page)
                .args([
                    "-thumbnail",
                    "384x384",
                    "-background",
                    "white",
                    "-alpha",
                    "remove",
                    "-alpha",
                    "off",
                ])
                .arg(&output),
        )
        .map_err(|e| format!("No se pudo iniciar convert: {e}"))?;
        if status.success() && output.exists() {
            return Ok(Some(path_to_string_fast(&output)));
        }
    }

    mark_thumbnail_failed(&source);
    Ok(None)
}

#[tauri::command]
pub fn get_thumbnail_data_url(path: String) -> Result<Option<String>, String> {
    const MAX_IMAGE_BYTES: u64 = 48 * 1024 * 1024;
    let source = PathBuf::from(&path);

    if let Some(mime) = image_mime(&source) {
        let meta = fs::metadata(&source).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES {
            return Ok(None);
        }
        let bytes = fs::read(&source).map_err(|e| e.to_string())?;
        return Ok(Some(format!(
            "data:{mime};base64,{}",
            base64_encode(&bytes)
        )));
    }

    if extension_lower(&source) == "pdf" {
        let Some(thumbnail_path) = get_pdf_thumbnail(path)? else {
            return Ok(None);
        };
        let bytes = fs::read(thumbnail_path).map_err(|e| e.to_string())?;
        return Ok(Some(format!(
            "data:image/png;base64,{}",
            base64_encode(&bytes)
        )));
    }

    Ok(None)
}

#[tauri::command]
pub fn get_properties(path: String) -> Result<PathProperties, String> {
    let p = PathBuf::from(&path);
    let meta = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
    let is_symlink = meta.file_type().is_symlink();
    let is_dir = if is_symlink {
        fs::metadata(&p).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        meta.is_dir()
    };

    let size = if is_dir { 0 } else { meta.len() };

    let kind = if is_dir {
        "Folder".into()
    } else if is_symlink {
        "Symlink".into()
    } else {
        "File".into()
    };
    let (kind_label, mime_type) = file_type_details(&p, is_dir, is_symlink);
    let extension = p
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase());

    let mode = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", meta.permissions().mode() & 0o777)
        }
        #[cfg(not(unix))]
        {
            "—".into()
        }
    };
    let readonly = meta.permissions().readonly();

    #[cfg(unix)]
    let (owner, group) = {
        use std::os::unix::fs::MetadataExt;
        (Some(meta.uid()), Some(meta.gid()))
    };
    #[cfg(not(unix))]
    let (owner, group) = (None, None);

    Ok(PathProperties {
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path_to_string_fast(&p),
        kind,
        kind_label,
        extension,
        mime_type,
        size,
        size_display: if is_dir {
            "—".into()
        } else {
            format_size(size)
        },
        created: ts_secs(&p, |m| m.created()),
        modified: ts_secs(&p, |m| m.modified()),
        accessed: ts_secs(&p, |m| m.accessed()),
        permissions: mode,
        readonly,
        owner,
        group,
        is_symlink,
        symlink_target: if is_symlink {
            fs::read_link(&p)
                .ok()
                .map(|target| path_to_string_fast(&target))
        } else {
            None
        },
        item_count: if is_dir { count_dir_items(&p) } else { None },
    })
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn open_terminal_here(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(err("No es una carpeta"));
    }
    if which("foot") {
        Command::new("foot")
            .args(["-D", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("kitty") {
        Command::new("kitty")
            .args(["--directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("alacritty") {
        Command::new("alacritty")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("ghostty") {
        Command::new("ghostty")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("wezterm") {
        Command::new("wezterm")
            .args(["start", "--cwd", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if which("gnome-terminal") {
        Command::new("gnome-terminal")
            .args(["--working-directory", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err(err("No se encontró una terminal compatible"))
}

#[tauri::command]
pub fn startup_bundle() -> Result<StartupBundle, String> {
    crate::platform::debug_log("startup_bundle");
    let dirs = get_special_dirs_cached()?;
    let home = dirs.home.clone();
    let platform = crate::platform::get_platform_info();
    let sidebar_items = crate::sidebar::build_sidebar_items()?;
    let localsend = crate::localsend::detect_localsend();
    let folder_customizations =
        crate::folder_customization::load_customizations_map().unwrap_or_default();

    Ok(StartupBundle {
        home,
        initial_path: initial_launch_path(),
        platform,
        sidebar_items,
        localsend_available: localsend.available,
        folder_customizations,
        special_dirs: dirs,
    })
}
