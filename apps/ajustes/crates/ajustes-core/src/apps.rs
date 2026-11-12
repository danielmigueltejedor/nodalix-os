//! The kinds of default application the Apps page lets you choose. Each one is a handful of
//! content types set together, so "Web" changes http, https and HTML in one go.

#[derive(Clone, Copy, Debug)]
pub struct Default {
    pub title: &'static str,
    pub icon: &'static str,
    /// Content types (or URL scheme handlers) the choice applies to; the first one is used to
    /// find the candidates and to read the current default.
    pub types: &'static [&'static str],
}

pub const DEFAULTS: &[Default] = &[
    Default {
        title: "Web",
        icon: "web-browser-symbolic",
        types: &[
            "x-scheme-handler/http",
            "x-scheme-handler/https",
            "text/html",
        ],
    },
    Default {
        title: "Mail",
        icon: "mail-unread-symbolic",
        types: &["x-scheme-handler/mailto"],
    },
    Default {
        title: "Calendar",
        icon: "x-office-calendar-symbolic",
        types: &["text/calendar"],
    },
    Default {
        title: "Music",
        icon: "audio-x-generic-symbolic",
        types: &["audio/mpeg", "audio/flac", "audio/ogg"],
    },
    Default {
        title: "Video",
        icon: "video-x-generic-symbolic",
        types: &["video/mp4", "video/x-matroska", "video/webm"],
    },
    Default {
        title: "Photos",
        icon: "image-x-generic-symbolic",
        types: &["image/png", "image/jpeg", "image/webp"],
    },
    Default {
        title: "Documents",
        icon: "x-office-document-symbolic",
        types: &["application/pdf"],
    },
    Default {
        title: "Files",
        icon: "folder-symbolic",
        types: &["inode/directory"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_names_at_least_one_content_type() {
        for d in DEFAULTS {
            assert!(!d.types.is_empty(), "{} has no types", d.title);
            assert!(d.types.iter().all(|t| t.contains('/')), "{}", d.title);
        }
    }

    #[test]
    fn no_content_type_belongs_to_two_defaults() {
        let mut seen = std::collections::HashSet::new();
        for d in DEFAULTS {
            for t in d.types {
                assert!(seen.insert(*t), "{t} is listed twice");
            }
        }
    }
}
