//! The icons of files: one page for all of them, with a folded corner, a small glyph for what is
//! inside and a pill with the extension, tinted by the kind of file. They are drawn here, like the
//! folders, so every file in Archivador looks like one family whatever icon theme the system has.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use ndl::gtk::{gio, glib, prelude::*};
use nodalia_gtk as ndl;

/// Bump when a drawing changes, so a copy cached by an older version is not reused.
const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Pdf,
    Document,
    Text,
    Sheet,
    Slides,
    Image,
    Audio,
    Video,
    Archive,
    Code,
    Disk,
    Package,
    Generic,
}

impl Category {
    #[cfg(test)]
    const ALL: [Category; 13] = [
        Category::Pdf,
        Category::Document,
        Category::Text,
        Category::Sheet,
        Category::Slides,
        Category::Image,
        Category::Audio,
        Category::Video,
        Category::Archive,
        Category::Code,
        Category::Disk,
        Category::Package,
        Category::Generic,
    ];

    fn name(self) -> &'static str {
        match self {
            Category::Pdf => "pdf",
            Category::Document => "document",
            Category::Text => "text",
            Category::Sheet => "sheet",
            Category::Slides => "slides",
            Category::Image => "image",
            Category::Audio => "audio",
            Category::Video => "video",
            Category::Archive => "archive",
            Category::Code => "code",
            Category::Disk => "disk",
            Category::Package => "package",
            Category::Generic => "generic",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Category::Pdf => "#e5484d",
            Category::Document => "#3e7bfa",
            Category::Text => "#6b7488",
            Category::Sheet => "#2fa36b",
            Category::Slides => "#f28c38",
            Category::Image => "#14a6b5",
            Category::Audio => "#e0508b",
            Category::Video => "#7c5cf0",
            Category::Archive => "#d99a1b",
            Category::Code => "#5663f5",
            Category::Disk => "#64748b",
            Category::Package => "#c27c2c",
            Category::Generic => "#8590a6",
        }
    }

    /// What is drawn above the pill, on a 24 px grid, as strokes in the category's colour.
    fn glyph(self) -> &'static str {
        match self {
            Category::Pdf => {
                r#"<path d="M5 17c3-1 5-5 6-10 .3-1.600 1.700-1.600 1.800 0 .3 3 2.600 7.500 6.200 9 .8.400.600 1.600-.600 1.600-3 0-7.500-.8-10.500 1.200C6 19.800 4.200 18 5 17z"/>"#
            }
            Category::Document => r#"<path d="M5 6h14M5 10.500h14M5 15h14M5 19.500h8"/>"#,
            Category::Text => r#"<path d="M6 7h12M6 12h12M6 17h7"/>"#,
            Category::Sheet => {
                r#"<rect x="3.500" y="4.500" width="17" height="15" rx="2"/><path d="M3.500 10h17M3.500 14.800h17M9.500 4.500v15"/>"#
            }
            Category::Slides => {
                r#"<rect x="3.500" y="4.500" width="17" height="11.500" rx="2"/><path d="M12 16v3.500M8.500 19.500h7M7.500 12.500l3-3 2.500 2 3-3.500"/>"#
            }
            Category::Image => {
                r#"<rect x="3.500" y="4.500" width="17" height="15" rx="2"/><circle cx="8.500" cy="9.500" r="1.600"/><path d="M3.500 16.500l5-4.500 4 3.500 3-2.500 5 4"/>"#
            }
            Category::Audio => {
                r##"<g fill="#fff"><circle cx="7.500" cy="17.500" r="2.500"/><circle cx="17.500" cy="15.500" r="2.500"/></g><path d="M10 17.500V6.500l10-2v11"/>"##
            }
            Category::Video => {
                r##"<rect x="3.500" y="5" width="17" height="14" rx="2.500"/><path fill="currentColor" d="M10 9.200v5.600l4.800-2.800z"/>"##
            }
            Category::Archive => {
                r#"<rect x="3.500" y="4.500" width="17" height="4.500" rx="1.200"/><path d="M5 9v9.500A1.500 1.500 0 0 0 6.500 20h11a1.500 1.500 0 0 0 1.500-1.500V9M10 13h4"/>"#
            }
            Category::Code => {
                r#"<path d="M8.500 7.500 4 12l4.500 4.500M15.500 7.500 20 12l-4.500 4.500M13.500 5.500l-3 13"/>"#
            }
            Category::Disk => {
                r#"<circle cx="12" cy="12" r="8.500"/><circle cx="12" cy="12" r="2.500"/><path d="M12 3.500a8.500 8.500 0 0 1 6 2.500"/>"#
            }
            Category::Package => {
                r#"<path d="M12 3.500 20 8v8l-8 4.500L4 16V8zM4 8l8 4.500L20 8M12 12.500v8"/>"#
            }
            Category::Generic => r#"<path d="M7 9h10M7 13h10M7 17h6"/>"#,
        }
    }
}

const EXTENSIONS: &[(Category, &[&str])] = &[
    (Category::Pdf, &["pdf"]),
    (
        Category::Document,
        &[
            "doc", "docx", "odt", "rtf", "pages", "dotx", "dot", "wpd", "epub", "mobi",
        ],
    ),
    (
        Category::Text,
        &[
            "txt", "md", "markdown", "log", "rst", "tex", "nfo", "srt", "vtt",
        ],
    ),
    (
        Category::Sheet,
        &["xls", "xlsx", "ods", "csv", "tsv", "numbers", "xlsm"],
    ),
    (
        Category::Slides,
        &["ppt", "pptx", "odp", "key", "pps", "ppsx"],
    ),
    (
        Category::Image,
        &[
            "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tif", "tiff", "heic", "heif",
            "avif", "ico", "raw", "cr2", "nef", "dng", "psd", "xcf", "kra",
        ],
    ),
    (
        Category::Audio,
        &[
            "mp3", "flac", "wav", "ogg", "oga", "opus", "m4a", "aac", "wma", "aiff", "mid", "midi",
        ],
    ),
    (
        Category::Video,
        &[
            "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "ogv", "3gp",
        ],
    ),
    (
        Category::Archive,
        &[
            "zip", "tar", "gz", "tgz", "bz2", "xz", "zst", "7z", "rar", "lz", "lzma", "cab", "z",
        ],
    ),
    (
        Category::Code,
        &[
            "rs", "py", "js", "ts", "jsx", "tsx", "c", "h", "cpp", "hpp", "cc", "cs", "java", "go",
            "rb", "php", "swift", "kt", "lua", "sh", "fish", "bash", "zsh", "pl", "sql", "html",
            "htm", "css", "scss", "json", "yaml", "yml", "toml", "xml", "ini", "conf", "cfg",
            "ndl", "vue", "svelte", "nix", "mk", "cmake",
        ],
    ),
    (
        Category::Disk,
        &[
            "iso", "img", "dmg", "vdi", "vmdk", "qcow2", "vhd", "bin", "cue",
        ],
    ),
    (
        Category::Package,
        &[
            "deb", "rpm", "pkg", "apk", "flatpak", "snap", "appimage", "msi", "exe", "jar", "whl",
        ],
    ),
];

/// The extension of a file name, lowercase, if it has one that is plain.
fn extension(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty()
        && !ext.is_empty()
        && ext.len() <= 10
        && ext.chars().all(|c| c.is_ascii_alphanumeric()))
    .then(|| ext.to_ascii_lowercase())
}

/// What kind of file this is, from its extension and, failing that, its content type.
pub fn category(name: &str, content_type: &str) -> Category {
    if let Some(ext) = extension(name) {
        // `.tar.gz` and its kin end in the compressor, which is in the list.
        if let Some((category, _)) = EXTENSIONS
            .iter()
            .find(|(_, list)| list.contains(&ext.as_str()))
        {
            return *category;
        }
    }
    match content_type {
        "application/pdf" => Category::Pdf,
        t if t.starts_with("image/") => Category::Image,
        t if t.starts_with("audio/") => Category::Audio,
        t if t.starts_with("video/") => Category::Video,
        t if t.starts_with("text/x-") || t == "application/json" || t == "application/xml" => {
            Category::Code
        }
        t if t.starts_with("text/") => Category::Text,
        t if t.contains("zip") || t.contains("compressed") || t.contains("tar") => {
            Category::Archive
        }
        t if t.contains("wordprocessing") || t.contains("msword") => Category::Document,
        t if t.contains("spreadsheet") || t.contains("excel") => Category::Sheet,
        t if t.contains("presentation") || t.contains("powerpoint") => Category::Slides,
        t if t.contains("iso9660") || t.contains("disk-image") => Category::Disk,
        _ => Category::Generic,
    }
}

/// The text in the pill: the extension in capitals, when it is short enough to read.
pub fn badge(name: &str) -> Option<String> {
    extension(name)
        .filter(|ext| ext.len() <= 4)
        .map(|ext| ext.to_ascii_uppercase())
}

/// The drawing, on a 128 px grid: a white page with a folded corner, the glyph of its kind, and
/// the pill under it.
fn drawing(category: Category, badge: Option<&str>) -> String {
    let color = category.color();
    let glyph = category
        .glyph()
        .replace("currentColor", color)
        .replace("#fff", "#ffffff");
    let glyph_y = if badge.is_some() { 24.0 } else { 34.0 };
    let pill = badge.map_or(String::new(), |text| {
        let width = 22.0 + 10.5 * text.len() as f32;
        let x = 64.0 - width / 2.0;
        format!(
            r##"<rect x="{x}" y="90" width="{width}" height="21" rx="10.5" fill="{color}"/>
<text x="64" y="105.2" text-anchor="middle" font-family="Adwaita Sans, Cantarell, Inter, sans-serif" font-size="13" font-weight="700" letter-spacing="0.6" fill="#ffffff">{text}</text>"##
        )
    });
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
<defs>
<linearGradient id="page" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#eef1f7"/></linearGradient>
<filter id="soft" x="-30%" y="-20%" width="160%" height="150%"><feGaussianBlur stdDeviation="3"/></filter>
</defs>
<rect x="28" y="12" width="72" height="112" rx="10" fill="#1e2433" opacity="0.22" filter="url(#soft)"/>
<path fill="url(#page)" stroke="#d3d9e6" stroke-width="1.5" d="M34 6h42l28 28v78a8 8 0 0 1-8 8H34a8 8 0 0 1-8-8V14a8 8 0 0 1 8-8z"/>
<path fill="{color}" fill-opacity="0.16" stroke="#d3d9e6" stroke-width="1.5" stroke-linejoin="round" d="M76 6v20a8 8 0 0 0 8 8h20z"/>
<g transform="translate(37 {glyph_y}) scale(2.25)" fill="none" stroke="{color}" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">{glyph}</g>
{pill}
</svg>
"##
    )
}

fn cache_dir() -> PathBuf {
    glib::user_cache_dir()
        .join("nodalix")
        .join("archivador")
        .join("files")
}

thread_local! {
    static ICONS: RefCell<HashMap<(Category, Option<String>), gio::Icon>> = RefCell::new(HashMap::new());
}

/// The icon of a file with this name and content type. The drawings are written under the user's
/// cache the first time they are needed, since an icon is looked up from a file.
pub fn icon(name: &str, content_type: &str) -> gio::Icon {
    let category = category(name, content_type);
    let badge = badge(name);
    ICONS.with(|icons| {
        icons
            .borrow_mut()
            .entry((category, badge.clone()))
            .or_insert_with(|| {
                let file = cache_dir().join(format!(
                    "{}-{}-v{VERSION}.svg",
                    category.name(),
                    badge.as_deref().unwrap_or("none").to_ascii_lowercase()
                ));
                if !file.exists() {
                    let _ = std::fs::create_dir_all(cache_dir());
                    let _ = std::fs::write(&file, drawing(category, badge.as_deref()));
                }
                gio::FileIcon::new(&gio::File::for_path(file)).upcast()
            })
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_decide_the_kind() {
        assert_eq!(category("report.PDF", ""), Category::Pdf);
        assert_eq!(category("notes.md", ""), Category::Text);
        assert_eq!(category("main.rs", ""), Category::Code);
        assert_eq!(category("photo.jpeg", ""), Category::Image);
        assert_eq!(category("backup.tar.gz", ""), Category::Archive);
        assert_eq!(category("disk.iso", ""), Category::Disk);
        assert_eq!(category("song.flac", ""), Category::Audio);
    }

    #[test]
    fn the_content_type_decides_when_the_name_does_not() {
        assert_eq!(category("README", "text/plain"), Category::Text);
        assert_eq!(category("script", "text/x-python"), Category::Code);
        assert_eq!(category("clip", "video/mp4"), Category::Video);
        assert_eq!(
            category("thing", "application/octet-stream"),
            Category::Generic
        );
        assert_eq!(category(".bashrc", "text/plain"), Category::Text);
    }

    #[test]
    fn the_pill_shows_short_plain_extensions_only() {
        assert_eq!(badge("a.pdf").as_deref(), Some("PDF"));
        assert_eq!(badge("a.docx").as_deref(), Some("DOCX"));
        assert_eq!(badge("a.flatpak"), None);
        assert_eq!(badge("README"), None);
        assert_eq!(badge(".hidden"), None);
        assert_eq!(badge("a.<b>"), None);
    }

    #[test]
    fn every_category_draws_a_wellformed_page() {
        for category in Category::ALL {
            for badge in [None, Some("TXT")] {
                let svg = drawing(category, badge);
                assert!(svg.contains("</svg>"), "{category:?}");
                assert_eq!(svg.matches("<g ").count(), svg.matches("</g>").count());
                assert_eq!(svg.contains("<text"), badge.is_some());
                assert!(!svg.contains("currentColor"));
            }
        }
        let mut names: Vec<_> = Category::ALL.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Category::ALL.len());
    }
}
