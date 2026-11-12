//! The folder icons: Nodalia's own, a teal folder with an NDL glyph on its front, drawn here
//! because NDL itself only has symbolic glyphs and the system's coloured folders (Adwaita's
//! blue ones) would not belong to it.
//!
//! The colours are NDL's accent, fixed in the drawing: an icon is a picture, not a style sheet,
//! and teal holds up on a light window and on a dark one.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use ndl::gtk::{gio, glib, prelude::*};
use nodalia_gtk as ndl;

/// Bump when a drawing changes, so a copy cached by an older version is not reused.
const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Plain,
    Documents,
    Downloads,
    Music,
    Pictures,
    Videos,
    Desktop,
    Templates,
    Public,
    Applications,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Plain => "folder",
            Kind::Documents => "folder-documents",
            Kind::Downloads => "folder-download",
            Kind::Music => "folder-music",
            Kind::Pictures => "folder-pictures",
            Kind::Videos => "folder-videos",
            Kind::Desktop => "user-desktop",
            Kind::Templates => "folder-templates",
            Kind::Public => "folder-publicshare",
            Kind::Applications => "folder-applications",
        }
    }

    /// What is drawn on the front, on NDL's 24 px grid, white. `None` for a plain folder.
    fn glyph(self) -> Option<&'static str> {
        Some(match self {
            Kind::Plain => return None,
            Kind::Documents => {
                r#"<path d="M13.5 3h-6A1.5 1.5 0 0 0 6 4.5v15A1.5 1.5 0 0 0 7.5 21h9a1.5 1.5 0 0 0 1.5-1.5v-12zM13.5 3v4.5H18M9 12h6M9 16h4"/>"#
            }
            Kind::Downloads => {
                r#"<path d="M12 3.5v11M7.5 10.5 12 15l4.5-4.5M4 16v3a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-3"/>"#
            }
            Kind::Music => {
                r##"<g fill="#fff"><circle cx="7.5" cy="17.5" r="2.5"/><circle cx="17.5" cy="15.5" r="2.5"/></g><path d="M10 17.5V6l10-2v11.5"/>"##
            }
            Kind::Pictures => {
                r#"<rect x="3.5" y="5" width="17" height="14" rx="1.5"/><circle cx="8.5" cy="9.5" r="1.5"/><path d="M3.5 16 8 12l4 3.5 3-2.5 5.5 4.5"/>"#
            }
            Kind::Videos => {
                r#"<rect x="3.5" y="4.5" width="17" height="15" rx="1.5"/><path d="M7.5 4.5v15M16.5 4.5v15M3.5 9.5h4M3.5 14.5h4M16.5 9.5h4M16.5 14.5h4"/>"#
            }
            Kind::Desktop => {
                r#"<rect x="3.5" y="4.5" width="17" height="12" rx="1.5"/><path d="M12 16.5V20M8 20h8"/>"#
            }
            Kind::Templates => {
                r#"<rect x="3.5" y="4.5" width="17" height="4.5" rx="1"/><path d="M5 9v9.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V9M10 13h4"/>"#
            }
            Kind::Public => {
                r#"<circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8.2 10.9l7.6-3.8M8.2 13.1l7.6 3.8"/>"#
            }
            Kind::Applications => {
                r##"<g fill="#fff"><rect x="4" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5"/></g>"##
            }
        })
    }
}

/// The drawing, on a 128 px grid: the back of the folder with its tab, the front in a lighter
/// teal, and the glyph in the middle of the front.
fn drawing(kind: Kind) -> String {
    let glyph = kind.glyph().map_or(String::new(), |g| {
        format!(
            r##"<g transform="translate(43 54) scale(1.75)" fill="none" stroke="#ffffff" stroke-opacity="0.96" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round">{g}</g>"##
        )
    });
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
<defs>
<linearGradient id="back" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0d8fa0"/><stop offset="1" stop-color="#086770"/></linearGradient>
<linearGradient id="front" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3dd0dd"/><stop offset="1" stop-color="#0a8f9e"/></linearGradient>
</defs>
<path fill="url(#back)" d="M10 30a8 8 0 0 1 8-8h28.5a8 8 0 0 1 5.7 2.4L60 32h50a8 8 0 0 1 8 8v61a8 8 0 0 1-8 8H18a8 8 0 0 1-8-8z"/>
<path fill="url(#front)" d="M10 52a8 8 0 0 1 8-8h92a8 8 0 0 1 8 8v49a8 8 0 0 1-8 8H18a8 8 0 0 1-8-8z"/>
<path fill="none" stroke="#ffffff" stroke-opacity="0.35" stroke-width="1.5" d="M18.5 44.75h91"/>
{glyph}
</svg>
"##
    )
}

fn cache_dir() -> PathBuf {
    glib::user_cache_dir()
        .join("nodalix")
        .join("archivador")
        .join("folders")
}

thread_local! {
    static ICONS: RefCell<HashMap<Kind, gio::Icon>> = RefCell::new(HashMap::new());
}

/// The icon of a folder of this kind. The drawings are written under the user's cache the first
/// time they are needed, since an icon is looked up from a file.
pub fn icon(kind: Kind) -> gio::Icon {
    ICONS.with(|icons| {
        icons
            .borrow_mut()
            .entry(kind)
            .or_insert_with(|| {
                let file = cache_dir().join(format!("{}-v{VERSION}.svg", kind.name()));
                if !file.exists() {
                    let _ = std::fs::create_dir_all(cache_dir());
                    let _ = std::fs::write(&file, drawing(kind));
                }
                gio::FileIcon::new(&gio::File::for_path(file)).upcast()
            })
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Kind; 10] = [
        Kind::Plain,
        Kind::Documents,
        Kind::Downloads,
        Kind::Music,
        Kind::Pictures,
        Kind::Videos,
        Kind::Desktop,
        Kind::Templates,
        Kind::Public,
        Kind::Applications,
    ];

    #[test]
    fn every_kind_has_a_name_of_its_own_and_a_wellformed_drawing() {
        let mut names: Vec<_> = ALL.iter().map(|k| k.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ALL.len());
        for kind in ALL {
            let svg = drawing(kind);
            assert!(svg.contains("</svg>"), "{kind:?}");
            assert_eq!(svg.matches("<g ").count(), svg.matches("</g>").count());
            // A plain folder has no glyph; every other one has.
            assert_eq!(kind.glyph().is_none(), kind == Kind::Plain, "{kind:?}");
        }
    }
}
