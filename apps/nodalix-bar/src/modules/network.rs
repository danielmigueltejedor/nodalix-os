pub fn label() -> String {
    super::capture("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .and_then(|rows| {
            rows.lines()
                .find_map(|line| line.strip_prefix("yes:").map(str::to_string))
        })
        .filter(|ssid| !ssid.is_empty())
        .map(|ssid| format!("󰤨 {ssid}"))
        .unwrap_or_else(|| "󰤭".to_string())
}
