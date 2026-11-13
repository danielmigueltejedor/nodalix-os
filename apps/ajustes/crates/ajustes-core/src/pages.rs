//! The pages of Settings, in the order the sidebar shows them.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
}

const fn native(id: &'static str, title: &'static str, icon: &'static str) -> Page {
    Page { id, title, icon }
}

/// Sections of the sidebar: a title (empty for none) and its pages.
pub const SECTIONS: &[(&str, &[Page])] = &[
    (
        "",
        &[
            native("appearance", "Appearance", "applications-graphics-symbolic"),
            native("updates", "Updates", "software-update-available-symbolic"),
        ],
    ),
    (
        "Connections",
        &[
            native("network", "Network", "network-workgroup-symbolic"),
            native("wifi", "Wi-Fi", "network-wireless-symbolic"),
            native("bluetooth", "Bluetooth", "bluetooth-symbolic"),
            native("sharing", "Sharing", "preferences-system-sharing-symbolic"),
            native(
                "localsend",
                "LocalSend",
                "network-transmit-receive-symbolic",
            ),
            native("online-accounts", "Online accounts", "goa-panel-symbolic"),
            native("cloud", "Cloud storage", "folder-remote-symbolic"),
        ],
    ),
    (
        "Devices",
        &[
            native("display", "Displays", "video-display-symbolic"),
            native("sound", "Sound", "audio-volume-high-symbolic"),
            native("power", "Power", "battery-symbolic"),
            native("keyboard", "Keyboard", "input-keyboard-symbolic"),
            native("mouse", "Mouse and touchpad", "input-mouse-symbolic"),
            native("printers", "Printers", "printer-symbolic"),
            native("color", "Color", "preferences-color-symbolic"),
        ],
    ),
    (
        "System",
        &[
            native(
                "notifications",
                "Notifications",
                "preferences-system-notifications-symbolic",
            ),
            native("multitasking", "Multitasking", "view-grid-symbolic"),
            native("storage", "Storage", "drive-harddisk-symbolic"),
            native("applications", "Apps", "view-app-grid-symbolic"),
            native(
                "privacy",
                "Privacy and security",
                "preferences-system-privacy-symbolic",
            ),
            native(
                "accessibility",
                "Accessibility",
                "preferences-desktop-accessibility-symbolic",
            ),
            native(
                "datetime",
                "Date and time",
                "preferences-system-time-symbolic",
            ),
            native(
                "region",
                "Region and language",
                "preferences-desktop-locale-symbolic",
            ),
            native("users", "Users", "system-users-symbolic"),
            native("about", "About", "help-about-symbolic"),
        ],
    ),
];

/// Every page, flattened.
pub fn all() -> impl Iterator<Item = &'static Page> {
    SECTIONS.iter().flat_map(|(_, pages)| pages.iter())
}

pub fn find(id: &str) -> Option<&'static Page> {
    all().find(|p| p.id == id)
}

/// The page a name from the command line opens. It takes the page ids and the names of the
/// GNOME Control Center panels (`gnome-control-center wifi`, `org.gnome.Settings` links), so the
/// Shell, the notifications and the network indicator keep working when this app takes over.
/// `None` for a name nobody knows: the caller opens the first page.
pub fn resolve(name: &str) -> Option<&'static Page> {
    let name = name.trim().to_ascii_lowercase();
    if let Some(page) = find(&name) {
        return Some(page);
    }
    let id = match name.as_str() {
        "background" | "backgrounds" | "wallpaper" | "ubuntu" => "appearance",
        "wi-fi" | "wireless" => "wifi",
        "wwan" | "mobile" | "ethernet" | "vpn" | "proxy" | "connectivity" => "network",
        "displays" | "screen" | "night-light" => "display",
        "audio" => "sound",
        "battery" | "power-saving" => "power",
        "keyboard-shortcuts" | "shortcuts" | "input" | "region-input" => "keyboard",
        "touchpad" | "pointing" | "wacom" | "mouse-and-touchpad" => "mouse",
        "printer" | "printing" => "printers",
        "lock"
        | "location"
        | "camera"
        | "microphone"
        | "diagnostics"
        | "usb"
        | "thunderbolt"
        | "security"
        | "privacy-and-security" => "privacy",
        "default-apps" | "removable-media" | "apps" | "search" => "applications",
        "universal-access" | "a11y" => "accessibility",
        "date-time" | "time" | "date" | "clock" => "datetime",
        "info-overview" | "info" | "system" | "details" => "about",
        "language" | "locale" | "region-language" | "formats" => "region",
        "user-accounts" | "accounts" => "users",
        "goa" | "online" | "account" => "online-accounts",
        "share" | "remote-desktop" => "sharing",
        "storage-cloud" | "disks" => "storage",
        "workspaces" | "windows" | "multi-tasking" => "multitasking",
        "update" | "software-updates" => "updates",
        "notification" => "notifications",
        _ => return None,
    };
    find(id)
}

/// The page asked for on a command line (without the program name). It understands
/// `--page <name>`, `--page=<name>`, `-p <name>` and a bare `<name>`, as
/// `gnome-control-center` does; other options and the extra arguments a panel takes
/// (`wifi connect-hidden-network`) are ignored. `None` when it names no page.
pub fn from_args<S: AsRef<str>>(args: &[S]) -> Option<&'static Page> {
    let mut args = args.iter().map(AsRef::as_ref);
    while let Some(arg) = args.next() {
        let name = if let Some(name) = arg.strip_prefix("--page=") {
            name
        } else if arg == "--page" || arg == "-p" {
            args.next()?
        } else if arg.starts_with('-') {
            continue;
        } else {
            arg
        };
        return resolve(name);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn page_ids_are_unique() {
        let ids: HashSet<_> = all().map(|p| p.id).collect();
        assert_eq!(ids.len(), all().count());
    }

    #[test]
    fn finds_by_id() {
        assert_eq!(find("about").map(|p| p.id), Some("about"));
        assert_eq!(find("printers").map(|p| p.id), Some("printers"));
        assert!(find("nope").is_none());
    }

    #[test]
    fn the_first_page_is_appearance() {
        assert_eq!(SECTIONS[0].1[0].id, "appearance");
    }

    #[test]
    fn resolves_page_ids_and_gnome_panel_names() {
        let id = |name| resolve(name).map(|p| p.id);
        for page in all() {
            assert_eq!(id(page.id), Some(page.id));
        }
        assert_eq!(id("WiFi"), Some("wifi"));
        assert_eq!(id("background"), Some("appearance"));
        assert_eq!(id("universal-access"), Some("accessibility"));
        assert_eq!(id("info-overview"), Some("about"));
        assert_eq!(id("search"), Some("applications"));
        assert_eq!(id(" Bluetooth "), Some("bluetooth"));
        assert_eq!(id("no-such-panel"), None);
        assert_eq!(id(""), None);
    }

    #[test]
    fn every_alias_lands_on_a_page() {
        // `resolve` finds the page with `find(id)`, so an alias that names no page would show
        // up as `None` here. The names the Shell and the GNOME desktop files use:
        for name in [
            "wifi",
            "network",
            "bluetooth",
            "display",
            "sound",
            "power",
            "keyboard",
            "mouse",
            "printers",
            "color",
            "privacy",
            "applications",
            "notifications",
            "search",
            "multitasking",
            "datetime",
            "about",
            "users",
            "online-accounts",
            "sharing",
            "region",
            "background",
            "wwan",
            "usb",
            "thunderbolt",
            "lock",
            "location",
            "camera",
            "microphone",
            "default-apps",
            "removable-media",
            "universal-access",
            "info-overview",
            "diagnostics",
            "connectivity",
            "wacom",
            "ubuntu",
        ] {
            assert!(resolve(name).is_some(), "{name} opens nothing");
        }
    }

    #[test]
    fn reads_the_page_from_a_command_line() {
        let id = |args: &[&str]| from_args(args).map(|p| p.id);
        assert_eq!(id(&["wifi"]), Some("wifi"));
        assert_eq!(id(&["--page", "bluetooth"]), Some("bluetooth"));
        assert_eq!(id(&["--page=power"]), Some("power"));
        assert_eq!(id(&["-p", "sound"]), Some("sound"));
        assert_eq!(id(&["--verbose", "network", "wifi"]), Some("network"));
        assert_eq!(id(&["wifi", "connect-hidden-network"]), Some("wifi"));
        assert_eq!(id(&["nonsense"]), None);
        assert_eq!(id(&["--page"]), None);
        assert_eq!(id(&["--verbose"]), None);
        assert_eq!(id(&[]), None);
    }
}
