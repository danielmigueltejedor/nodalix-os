//! The pages of Settings, in the order the sidebar shows them.

/// How a page is provided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Built into this app.
    Native,
    /// Still the GNOME Control Center panel with this name, opened in its own window until it is
    /// ported.
    Legacy(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub kind: Kind,
}

const fn native(id: &'static str, title: &'static str, icon: &'static str) -> Page {
    Page {
        id,
        title,
        icon,
        kind: Kind::Native,
    }
}

const fn legacy(
    id: &'static str,
    title: &'static str,
    icon: &'static str,
    panel: &'static str,
) -> Page {
    Page {
        id,
        title,
        icon,
        kind: Kind::Legacy(panel),
    }
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
            legacy(
                "network",
                "Network",
                "network-workgroup-symbolic",
                "network",
            ),
            legacy("wifi", "Wi-Fi", "network-wireless-symbolic", "wifi"),
            legacy("bluetooth", "Bluetooth", "bluetooth-symbolic", "bluetooth"),
            legacy("sharing", "Sharing", "emblem-shared-symbolic", "sharing"),
            native(
                "localsend",
                "LocalSend",
                "network-transmit-receive-symbolic",
            ),
            legacy(
                "online-accounts",
                "Online accounts",
                "goa-panel-symbolic",
                "online-accounts",
            ),
        ],
    ),
    (
        "Devices",
        &[
            legacy("display", "Displays", "video-display-symbolic", "display"),
            legacy("sound", "Sound", "audio-volume-high-symbolic", "sound"),
            legacy("power", "Power", "battery-symbolic", "power"),
            legacy(
                "keyboard",
                "Keyboard",
                "input-keyboard-symbolic",
                "keyboard",
            ),
            legacy(
                "mouse",
                "Mouse and touchpad",
                "input-mouse-symbolic",
                "mouse",
            ),
            legacy("printers", "Printers", "printer-symbolic", "printers"),
            legacy("color", "Color", "preferences-color-symbolic", "color"),
        ],
    ),
    (
        "System",
        &[
            legacy(
                "notifications",
                "Notifications",
                "notification-symbolic",
                "notifications",
            ),
            legacy(
                "multitasking",
                "Multitasking",
                "view-grid-symbolic",
                "multitasking",
            ),
            legacy(
                "applications",
                "Apps",
                "view-app-grid-symbolic",
                "applications",
            ),
            legacy(
                "privacy",
                "Privacy and security",
                "preferences-system-privacy-symbolic",
                "privacy",
            ),
            legacy(
                "accessibility",
                "Accessibility",
                "preferences-desktop-accessibility-symbolic",
                "universal-access",
            ),
            legacy(
                "datetime",
                "Date and time",
                "preferences-system-time-symbolic",
                "datetime",
            ),
            legacy(
                "region",
                "Region and language",
                "preferences-desktop-locale-symbolic",
                "system",
            ),
            legacy("users", "Users", "system-users-symbolic", "users"),
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
        assert_eq!(find("about").map(|p| p.kind), Some(Kind::Native));
        assert_eq!(find("wifi").map(|p| p.kind), Some(Kind::Legacy("wifi")));
        assert!(find("nope").is_none());
    }

    #[test]
    fn panels_are_plain_names_a_command_line_can_take() {
        for page in all() {
            if let Kind::Legacy(panel) = page.kind {
                assert!(
                    panel.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                    "{panel} is not a panel name"
                );
            }
        }
    }

    #[test]
    fn the_native_pages_come_first() {
        assert_eq!(SECTIONS[0].1[0].id, "appearance");
        assert!(SECTIONS[0].1.iter().all(|p| p.kind == Kind::Native));
    }
}
