//! Wi-Fi networks as NetworkManager describes them, without any D-Bus: what is secured, how
//! strong a signal looks and which access points to show.

/// `NM_802_11_AP_FLAGS_PRIVACY`: the network needs a key.
const AP_FLAGS_PRIVACY: u32 = 0x1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPoint {
    /// The object path NetworkManager gives it; what a connection request names.
    pub path: String,
    pub ssid: String,
    /// 0 to 100.
    pub strength: u8,
    pub secured: bool,
    /// WPA with a login (802.1X), which needs more than a password.
    pub enterprise: bool,
}

/// A network name is bytes with no promised encoding.
pub fn ssid_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Whether a network asks for a key, from its three flag words.
pub fn is_secured(flags: u32, wpa_flags: u32, rsn_flags: u32) -> bool {
    flags & AP_FLAGS_PRIVACY != 0 || wpa_flags != 0 || rsn_flags != 0
}

/// `NM_802_11_AP_SEC_KEY_MGMT_802_1X`.
const KEY_MGMT_802_1X: u32 = 0x200;

/// Whether a network uses a login (WPA enterprise) instead of a shared password.
pub fn is_enterprise(wpa_flags: u32, rsn_flags: u32) -> bool {
    (wpa_flags | rsn_flags) & KEY_MGMT_802_1X != 0
}

/// The symbolic icon for a signal strength.
pub fn signal_icon(strength: u8) -> &'static str {
    match strength {
        80.. => "network-wireless-signal-excellent-symbolic",
        55..=79 => "network-wireless-signal-good-symbolic",
        30..=54 => "network-wireless-signal-ok-symbolic",
        1..=29 => "network-wireless-signal-weak-symbolic",
        0 => "network-wireless-signal-none-symbolic",
    }
}

/// The networks to list: hidden ones (no name) left out, one entry per name (the strongest),
/// strongest first and then by name.
pub fn visible(mut points: Vec<AccessPoint>) -> Vec<AccessPoint> {
    points.retain(|p| !p.ssid.is_empty());
    // Strongest of each name first, so keeping the first of a name keeps that one.
    points.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    let mut seen = std::collections::HashSet::new();
    points.retain(|p| seen.insert(p.ssid.clone()));
    points
}

/// A WPA passphrase is 8 to 63 characters; a 64-digit hex string is a raw key.
pub fn password_is_valid(password: &str) -> bool {
    let n = password.chars().count();
    (8..=63).contains(&n) || (n == 64 && password.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ap(path: &str, ssid: &str, strength: u8) -> AccessPoint {
        AccessPoint {
            path: path.into(),
            ssid: ssid.into(),
            strength,
            secured: true,
            enterprise: false,
        }
    }

    #[test]
    fn a_network_is_secured_by_any_of_the_flags() {
        assert!(!is_secured(0, 0, 0));
        assert!(is_secured(1, 0, 0));
        assert!(is_secured(0, 0x100, 0));
        assert!(is_secured(0, 0, 0x100));
    }

    #[test]
    fn a_login_network_is_enterprise() {
        assert!(is_enterprise(0, 0x200));
        assert!(is_enterprise(0x200, 0));
        assert!(!is_enterprise(0x100, 0x100));
    }

    #[test]
    fn the_signal_icon_follows_the_strength() {
        assert!(signal_icon(100).contains("excellent"));
        assert!(signal_icon(60).contains("good"));
        assert!(signal_icon(40).contains("ok"));
        assert!(signal_icon(10).contains("weak"));
        assert!(signal_icon(0).contains("none"));
    }

    #[test]
    fn lists_one_entry_per_name_the_strongest_first() {
        let list = visible(vec![
            ap("/a", "Casa", 40),
            ap("/b", "", 90),
            ap("/c", "Casa", 70),
            ap("/d", "Bar", 70),
            ap("/e", "Zeta", 20),
        ]);
        let names: Vec<_> = list
            .iter()
            .map(|p| (p.ssid.as_str(), p.path.as_str()))
            .collect();
        assert_eq!(names, [("Bar", "/d"), ("Casa", "/c"), ("Zeta", "/e")]);
    }

    #[test]
    fn a_name_that_is_not_utf8_still_shows() {
        assert_eq!(ssid_text(b"Caf\xe9"), "Caf\u{fffd}");
        assert_eq!(ssid_text("Wi-Fi ñ".as_bytes()), "Wi-Fi ñ");
    }

    #[test]
    fn checks_the_length_of_a_passphrase() {
        assert!(!password_is_valid("short"));
        assert!(password_is_valid("12345678"));
        assert!(password_is_valid(&"a".repeat(63)));
        // 64 characters are a raw key only when they are all hexadecimal.
        assert!(!password_is_valid(&"g".repeat(64)));
        assert!(password_is_valid(&"f".repeat(64)));
    }
}
