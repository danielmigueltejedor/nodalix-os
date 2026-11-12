//! Keyboard layouts (from xkb's `evdev.lst`), the compose key and the shortcut table.

/// A layout the system offers, with the variants it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub code: String,
    pub name: String,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub code: String,
    pub name: String,
}

/// The layouts and variants of an `evdev.lst`: the `! layout` section lists `code  Name`, the
/// `! variant` section `code  layout: Name`.
pub fn parse_layouts(lst: &str) -> Vec<Layout> {
    let mut layouts: Vec<Layout> = Vec::new();
    let mut section = "";
    for line in lst.lines() {
        if let Some(name) = line.strip_prefix("! ") {
            section = name.trim();
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((code, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim();
        match section {
            // The user-defined layout is a placeholder, not something to pick.
            "layout" if code == "custom" => {}
            "layout" => layouts.push(Layout {
                code: code.to_owned(),
                name: rest.to_owned(),
                variants: Vec::new(),
            }),
            "variant" => {
                let Some((layout, name)) = rest.split_once(": ") else {
                    continue;
                };
                if let Some(l) = layouts.iter_mut().find(|l| l.code == layout) {
                    l.variants.push(Variant {
                        code: code.to_owned(),
                        name: name.trim().to_owned(),
                    });
                }
            }
            _ => {}
        }
    }
    layouts.sort_by_key(|l| l.name.to_lowercase());
    layouts
}

/// The id GNOME stores for a layout and an optional variant: `es` or `es+dvorak`.
pub fn source_id(layout: &str, variant: &str) -> String {
    if variant.is_empty() {
        layout.to_owned()
    } else {
        format!("{layout}+{variant}")
    }
}

/// What to call a stored source: its layout's name and its variant's, or the id itself when the
/// system does not list it.
pub fn describe(layouts: &[Layout], id: &str) -> String {
    let (code, variant) = id.split_once('+').unwrap_or((id, ""));
    let Some(layout) = layouts.iter().find(|l| l.code == code) else {
        return id.to_owned();
    };
    match layout.variants.iter().find(|v| v.code == variant) {
        _ if variant.is_empty() => layout.name.clone(),
        Some(v) => v.name.clone(),
        None => id.to_owned(),
    }
}

/// Moves the entry at `index` one place up; the first stays where it is.
pub fn move_up<T>(list: &mut [T], index: usize) {
    if index > 0 && index < list.len() {
        list.swap(index, index - 1);
    }
}

/// The compose key choices: label and the xkb option that sets it.
pub const COMPOSE_CHOICES: [(&str, &str); 6] = [
    ("None", ""),
    ("Right Alt", "compose:ralt"),
    ("Menu", "compose:menu"),
    ("Right Ctrl", "compose:rctrl"),
    ("Caps Lock", "compose:caps"),
    ("Right Super", "compose:rwin"),
];

/// The compose option among xkb options, if any.
pub fn compose_of(options: &[String]) -> Option<&str> {
    options
        .iter()
        .map(String::as_str)
        .find(|o| o.starts_with("compose:"))
}

/// The options with the compose key set to `choice` (an empty one removes it); everything else
/// stays as it was.
pub fn with_compose(options: &[String], choice: &str) -> Vec<String> {
    let mut out: Vec<String> = options
        .iter()
        .filter(|o| !o.starts_with("compose:"))
        .cloned()
        .collect();
    if !choice.is_empty() {
        out.push(choice.to_owned());
    }
    out
}

/// One shortcut that Settings lets a person change.
pub struct Shortcut {
    pub schema: &'static str,
    pub key: &'static str,
    pub title: &'static str,
    pub group: &'static str,
}

const WM: &str = "org.gnome.desktop.wm.keybindings";
const SHELL: &str = "org.gnome.shell.keybindings";
const MEDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";

/// The shortcuts offered, grouped; a key missing on the system is left out by the page.
pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        schema: SHELL,
        key: "toggle-overview",
        title: "Show the overview",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "switch-applications",
        title: "Switch applications",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "switch-windows",
        title: "Switch windows",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "switch-to-workspace-left",
        title: "Workspace to the left",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "switch-to-workspace-right",
        title: "Workspace to the right",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "show-desktop",
        title: "Show the desktop",
        group: "Navigation",
    },
    Shortcut {
        schema: WM,
        key: "close",
        title: "Close the window",
        group: "Windows",
    },
    Shortcut {
        schema: WM,
        key: "minimize",
        title: "Hide the window",
        group: "Windows",
    },
    Shortcut {
        schema: WM,
        key: "toggle-maximized",
        title: "Maximise the window",
        group: "Windows",
    },
    Shortcut {
        schema: WM,
        key: "toggle-fullscreen",
        title: "Full screen",
        group: "Windows",
    },
    Shortcut {
        schema: WM,
        key: "move-to-workspace-left",
        title: "Move the window to the left",
        group: "Windows",
    },
    Shortcut {
        schema: WM,
        key: "move-to-workspace-right",
        title: "Move the window to the right",
        group: "Windows",
    },
    Shortcut {
        schema: SHELL,
        key: "show-screenshot-ui",
        title: "Take a screenshot",
        group: "System",
    },
    Shortcut {
        schema: SHELL,
        key: "toggle-message-tray",
        title: "Show notifications",
        group: "System",
    },
    Shortcut {
        schema: MEDIA,
        key: "screensaver",
        title: "Lock the screen",
        group: "System",
    },
    Shortcut {
        schema: MEDIA,
        key: "terminal",
        title: "Open a terminal",
        group: "Launchers",
    },
    Shortcut {
        schema: MEDIA,
        key: "www",
        title: "Open the web browser",
        group: "Launchers",
    },
    Shortcut {
        schema: MEDIA,
        key: "home",
        title: "Open the home folder",
        group: "Launchers",
    },
    Shortcut {
        schema: MEDIA,
        key: "email",
        title: "Open the email",
        group: "Launchers",
    },
    Shortcut {
        schema: MEDIA,
        key: "volume-up",
        title: "Volume up",
        group: "Sound and media",
    },
    Shortcut {
        schema: MEDIA,
        key: "volume-down",
        title: "Volume down",
        group: "Sound and media",
    },
    Shortcut {
        schema: MEDIA,
        key: "volume-mute",
        title: "Mute",
        group: "Sound and media",
    },
    Shortcut {
        schema: MEDIA,
        key: "play",
        title: "Play or pause",
        group: "Sound and media",
    },
    Shortcut {
        schema: MEDIA,
        key: "next",
        title: "Next track",
        group: "Sound and media",
    },
    Shortcut {
        schema: MEDIA,
        key: "previous",
        title: "Previous track",
        group: "Sound and media",
    },
];

/// The shortcut that already uses `accelerator` among `taken` (title, accelerators) pairs,
/// other than the one at `except`.
pub fn conflict<T: Copy>(
    taken: &[(T, Vec<String>)],
    except: usize,
    accelerator: &str,
) -> Option<T> {
    taken
        .iter()
        .enumerate()
        .find(|(i, (_, keys))| *i != except && keys.iter().any(|k| k == accelerator))
        .map(|(_, (title, _))| *title)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LST: &str = "! model\n  pc86   Generic\n\n! layout\n  us   English (US)\n  es   Spanish\n\n! variant\n  dvorak   us: English (Dvorak)\n  deadtilde   es: Spanish (dead tilde)\n  ghost   zz: Nobody\n\n! option\n  compose:ralt   Right Alt\n";

    #[test]
    fn layouts_come_with_their_variants_sorted_by_name() {
        let layouts = parse_layouts(LST);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].code, "us");
        assert_eq!(layouts[1].code, "es");
        assert_eq!(layouts[1].variants[0].code, "deadtilde");
        assert_eq!(layouts[1].variants[0].name, "Spanish (dead tilde)");
    }

    #[test]
    fn the_custom_placeholder_is_not_offered() {
        let layouts =
            parse_layouts("! layout\n  custom   A user-defined custom Layout\n  es   Spanish\n");
        assert_eq!(layouts.len(), 1);
    }

    #[test]
    fn a_variant_of_an_unknown_layout_is_ignored() {
        let layouts = parse_layouts(LST);
        assert!(layouts
            .iter()
            .all(|l| l.variants.iter().all(|v| v.code != "ghost")));
    }

    #[test]
    fn ids_and_names_round_trip() {
        let layouts = parse_layouts(LST);
        assert_eq!(source_id("es", ""), "es");
        assert_eq!(source_id("es", "deadtilde"), "es+deadtilde");
        assert_eq!(describe(&layouts, "es"), "Spanish");
        assert_eq!(describe(&layouts, "es+deadtilde"), "Spanish (dead tilde)");
        assert_eq!(describe(&layouts, "fr"), "fr");
        assert_eq!(describe(&layouts, "es+nothing"), "es+nothing");
    }

    #[test]
    fn moving_up_swaps_with_the_previous_and_leaves_the_first_alone() {
        let mut v = vec![1, 2, 3];
        move_up(&mut v, 2);
        assert_eq!(v, [1, 3, 2]);
        move_up(&mut v, 0);
        move_up(&mut v, 9);
        assert_eq!(v, [1, 3, 2]);
    }

    #[test]
    fn the_compose_key_replaces_only_its_own_option() {
        let options = vec!["caps:escape".to_owned(), "compose:ralt".to_owned()];
        assert_eq!(compose_of(&options), Some("compose:ralt"));
        assert_eq!(
            with_compose(&options, "compose:menu"),
            ["caps:escape", "compose:menu"]
        );
        assert_eq!(with_compose(&options, ""), ["caps:escape"]);
        assert_eq!(compose_of(&["caps:escape".to_owned()]), None);
    }

    #[test]
    fn a_shortcut_in_use_is_named() {
        let taken = [
            ("Close", vec!["<Alt>F4".to_owned()]),
            ("Lock", vec!["<Super>l".to_owned()]),
        ];
        assert_eq!(conflict(&taken, 0, "<Super>l"), Some("Lock"));
        assert_eq!(conflict(&taken, 1, "<Super>l"), None);
        assert_eq!(conflict(&taken, 0, "<Super>x"), None);
    }

    #[test]
    fn every_shortcut_is_listed_once() {
        for (i, a) in SHORTCUTS.iter().enumerate() {
            assert!(SHORTCUTS[i + 1..]
                .iter()
                .all(|b| (a.schema, a.key) != (b.schema, b.key)));
        }
    }
}
