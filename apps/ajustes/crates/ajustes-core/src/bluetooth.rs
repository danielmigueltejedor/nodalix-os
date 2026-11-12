//! Bluetooth devices as BlueZ describes them, without any D-Bus: which to list and in what order.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// The object path BlueZ gives it.
    pub path: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    /// The freedesktop icon name BlueZ suggests (`audio-headphones`, `input-keyboard`…).
    pub icon: String,
    /// The charge in percent, when the device reports it (`org.bluez.Battery1`).
    pub battery: Option<u8>,
}

/// The icon for a charge: `battery-level-80-symbolic`, in steps of ten, never above a hundred.
pub fn battery_icon(percent: u8) -> String {
    let step = (u16::from(percent.min(100)) + 5) / 10 * 10;
    format!("battery-level-{step}-symbolic")
}

/// The symbolic icon for a device, from BlueZ's suggestion; a generic one when it has none.
pub fn icon_name(icon: &str) -> String {
    let valid = !icon.is_empty()
        && icon
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        format!("{icon}-symbolic")
    } else {
        "bluetooth-symbolic".to_string()
    }
}

/// Connected devices first, then the paired ones, then the rest; by name inside each group.
pub fn ordered(mut devices: Vec<Device>) -> Vec<Device> {
    devices.sort_by(|a, b| {
        let rank = |d: &Device| {
            if d.connected {
                0
            } else if d.paired {
                1
            } else {
                2
            }
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.path.cmp(&b.path))
    });
    devices
}

/// A device that is shown only if it has a name of its own: a nearby device that has not told
/// its name shows as its address, which is of no use to pick from.
pub fn has_real_name(name: &str, address: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.replace('-', ":") != address
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(path: &str, name: &str, paired: bool, connected: bool) -> Device {
        Device {
            path: path.into(),
            name: name.into(),
            paired,
            connected,
            icon: String::new(),
            battery: None,
        }
    }

    #[test]
    fn connected_first_then_paired_then_the_rest() {
        let list = ordered(vec![
            device("/d", "Zeta", false, false),
            device("/b", "beta", true, false),
            device("/a", "Alfa", true, true),
            device("/c", "Charlie", false, false),
        ]);
        let names: Vec<_> = list.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["Alfa", "beta", "Charlie", "Zeta"]);
    }

    #[test]
    fn icon_names_are_taken_only_when_plain() {
        assert_eq!(icon_name("audio-headphones"), "audio-headphones-symbolic");
        assert_eq!(icon_name(""), "bluetooth-symbolic");
        assert_eq!(icon_name("../x"), "bluetooth-symbolic");
        assert_eq!(icon_name("Audio"), "bluetooth-symbolic");
    }

    #[test]
    fn a_device_known_only_by_its_address_has_no_real_name() {
        assert!(has_real_name("AirPods", "AA:BB:CC:DD:EE:FF"));
        assert!(!has_real_name("AA-BB-CC-DD-EE-FF", "AA:BB:CC:DD:EE:FF"));
        assert!(!has_real_name("  ", "AA:BB:CC:DD:EE:FF"));
    }

    #[test]
    fn the_battery_icon_rounds_to_ten_and_stops_at_a_hundred() {
        assert_eq!(battery_icon(0), "battery-level-0-symbolic");
        assert_eq!(battery_icon(84), "battery-level-80-symbolic");
        assert_eq!(battery_icon(85), "battery-level-90-symbolic");
        assert_eq!(battery_icon(100), "battery-level-100-symbolic");
        assert_eq!(battery_icon(250), "battery-level-100-symbolic");
    }
}
