//! The folder icons: Nodalia's own, a flat teal folder with an NDL glyph tinted into its front, drawn here
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
const VERSION: u32 = 8;

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

    /// What is drawn on the front, on NDL's 24 px grid; `#fff` marks what is filled, and the
    /// stroke takes the glyph's tint. `None` for a plain folder.
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

/// The colours a folder can be given. The first is Nodalia's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Teal,
    Blue,
    Violet,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Graphite,
}

impl Color {
    pub const ALL: [Color; 9] = [
        Color::Teal,
        Color::Blue,
        Color::Violet,
        Color::Pink,
        Color::Red,
        Color::Orange,
        Color::Yellow,
        Color::Green,
        Color::Graphite,
    ];

    /// What is kept in the folder's `metadata::nodalix-folder-color`.
    pub fn id(self) -> &'static str {
        match self {
            Color::Teal => "teal",
            Color::Blue => "blue",
            Color::Violet => "violet",
            Color::Pink => "pink",
            Color::Red => "red",
            Color::Orange => "orange",
            Color::Yellow => "yellow",
            Color::Green => "green",
            Color::Graphite => "graphite",
        }
    }

    pub fn from_id(id: &str) -> Option<Color> {
        Color::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn title(self) -> &'static str {
        match self {
            Color::Teal => "Teal",
            Color::Blue => "Blue",
            Color::Violet => "Violet",
            Color::Pink => "Pink",
            Color::Red => "Red",
            Color::Orange => "Orange",
            Color::Yellow => "Yellow",
            Color::Green => "Green",
            Color::Graphite => "Graphite",
        }
    }

    /// Hue in degrees, and how saturated the colour is. They are kept soft: a folder is seen by
    /// the dozen, and nine loud colours side by side would shout.
    fn hue(self) -> (f32, f32) {
        match self {
            Color::Teal => (187.0, 0.52),
            Color::Blue => (214.0, 0.64),
            Color::Violet => (257.0, 0.52),
            Color::Pink => (332.0, 0.54),
            Color::Red => (358.0, 0.60),
            Color::Orange => (24.0, 0.72),
            Color::Yellow => (42.0, 0.80),
            Color::Green => (146.0, 0.40),
            Color::Graphite => (220.0, 0.09),
        }
    }

    /// The colour at this lightness (0 to 1), as `#rrggbb`.
    fn shade(self, lightness: f32) -> String {
        let (hue, saturation) = self.hue();
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let x = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = lightness - chroma / 2.0;
        let (r, g, b) = match (hue / 60.0) as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let byte = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b))
    }
}

/// The drawing, on a 128 px grid, flat and in two tones of one colour: the back of the folder
/// with its tab, the front a lighter tone with a hairline of light along its top, and the glyph in
/// the middle of the front in a deep tone. No shadows and no gloss: the shape does the work. A
/// folder that holds something shows two sheets of paper standing in it, above the front.
fn drawing(kind: Kind, color: Color, full: bool) -> String {
    let ink = color.shade(0.36);
    let glyph = kind.glyph().map_or(String::new(), |g| {
        let filled = g.replace("#fff", &ink);
        format!(
            r##"<g transform="translate(46 57) scale(1.5)" fill="none" stroke="{ink}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{filled}</g>"##
        )
    });
    let back = color.shade(0.47);
    let (front_top, front_bottom) = (color.shade(0.69), color.shade(0.63));
    let light = color.shade(0.78);
    let papers = if full {
        let edge = color.shade(0.40);
        format!(
            r##"<g stroke="{edge}" stroke-opacity="0.45" stroke-width="1.300">
<rect x="24" y="21" width="72" height="48" rx="4" fill="#eef1f6" transform="rotate(-4 60 45)"/>
<rect x="31" y="25" width="70" height="48" rx="4" fill="#ffffff"/>
</g>"##
        )
    } else {
        String::new()
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
<defs>
<linearGradient id="front" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{front_top}"/><stop offset="1" stop-color="{front_bottom}"/></linearGradient>
</defs>
<path fill="{back}" d="M15 32a10 10 0 0 1 10-10h21c3.600 0 6.400 1.300 8.800 4l3.400 3.800c2 2.200 4.300 3.200 7.300 3.200H103a10 10 0 0 1 10 10v53a10 10 0 0 1-10 10H25a10 10 0 0 1-10-10z"/>
{papers}
<path fill="url(#front)" d="M15 54a10 10 0 0 1 10-10h78a10 10 0 0 1 10 10v42a10 10 0 0 1-10 10H25a10 10 0 0 1-10-10z"/>
<path fill="none" stroke="{light}" stroke-width="1.500" stroke-linecap="round" d="M24 44.750h80"/>
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
    static ICONS: RefCell<HashMap<(Kind, Color, bool), gio::Icon>> = RefCell::new(HashMap::new());
}

/// The icon of a folder of this kind in this colour, with papers in it when it is `full`. The
/// drawings are written under the user's cache the first time they are needed, since an icon is
/// looked up from a file.
pub fn icon(kind: Kind, color: Color, full: bool) -> gio::Icon {
    ICONS.with(|icons| {
        icons
            .borrow_mut()
            .entry((kind, color, full))
            .or_insert_with(|| {
                let file = cache_dir().join(format!(
                    "{}-{}{}-v{VERSION}.svg",
                    kind.name(),
                    color.id(),
                    if full { "-full" } else { "" }
                ));
                if !file.exists() {
                    let _ = std::fs::create_dir_all(cache_dir());
                    let _ = std::fs::write(&file, drawing(kind, color, full));
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
    fn only_a_full_folder_has_papers_in_it() {
        let full = drawing(Kind::Plain, Color::Blue, true);
        let empty = drawing(Kind::Plain, Color::Blue, false);
        assert!(full.contains("<rect"));
        assert!(!empty.contains("<rect"));
        // The papers stand behind the front: drawn before it.
        assert!(full.find("<rect").unwrap() < full.find("url(#front)\"").unwrap());
    }

    #[test]
    fn every_kind_has_a_name_of_its_own_and_a_wellformed_drawing() {
        let mut names: Vec<_> = ALL.iter().map(|k| k.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ALL.len());
        for kind in ALL {
            let svg = drawing(kind, Color::Teal, true);
            assert!(svg.contains("</svg>"), "{kind:?}");
            assert_eq!(svg.matches("<g ").count(), svg.matches("</g>").count());
            // A plain folder has no glyph; every other one has.
            assert_eq!(kind.glyph().is_none(), kind == Kind::Plain, "{kind:?}");
        }
    }

    #[test]
    fn every_colour_draws_and_is_known_by_its_id() {
        for color in Color::ALL {
            assert_eq!(Color::from_id(color.id()), Some(color));
            let svg = drawing(Kind::Documents, color, false);
            assert!(svg.contains(&color.shade(0.69)));
            assert!(!svg.contains("NaN"));
        }
        assert_eq!(Color::from_id("magenta"), None);
        let mut ids: Vec<_> = Color::ALL.iter().map(|c| c.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Color::ALL.len());
    }

    #[test]
    fn shades_are_hex_and_lighter_means_lighter() {
        let dark = Color::Blue.shade(0.2);
        let light = Color::Blue.shade(0.8);
        assert_eq!(dark.len(), 7);
        assert!(dark.starts_with('#'));
        assert!(u32::from_str_radix(&dark[1..], 16).is_ok());
        let sum = |c: &str| {
            (1..7)
                .step_by(2)
                .map(|i| u32::from_str_radix(&c[i..i + 2], 16).unwrap())
                .sum::<u32>()
        };
        assert!(sum(&light) > sum(&dark));
        assert_eq!(Color::Teal.shade(1.0), "#ffffff");
    }
}
