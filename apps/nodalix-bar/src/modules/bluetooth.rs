pub fn label() -> String {
    let powered = super::capture("bluetoothctl", &["show"])
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.trim().strip_prefix("Powered: ").map(str::to_string))
        })
        .unwrap_or_default();
    if powered == "yes" {
        "󰂯".to_string()
    } else {
        "󰂲".to_string()
    }
}
