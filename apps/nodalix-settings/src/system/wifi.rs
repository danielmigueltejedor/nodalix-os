use super::{run_command_status, run_read_only};

#[derive(Debug, Clone)]
pub struct WifiStatus {
    pub radio_on: bool,
    pub connected: bool,
    pub ssid: String,
}

pub fn status() -> Result<WifiStatus, String> {
    let radio_on = wifi_radio_on().ok_or_else(|| "nmcli no disponible".to_string())?;
    let ssid = current_ssid();
    Ok(WifiStatus {
        radio_on,
        connected: ssid != "No conectado" && ssid != "—",
        ssid,
    })
}

pub fn wifi_radio_on() -> Option<bool> {
    let raw = run_read_only("nmcli", &["radio", "wifi"])?;
    Some(raw.trim() == "enabled")
}

pub fn set_wifi_radio(on: bool) -> Result<(), String> {
    let state = if on { "on" } else { "off" };
    run_command_status("nmcli", &["radio", "wifi", state]).map(|_| ())
}

pub fn current_ssid() -> String {
    let Some(output) = run_read_only("nmcli", &["-t", "-f", "active,ssid", "dev", "wifi"]) else {
        return "No disponible".to_string();
    };
    output
        .lines()
        .find_map(|line| line.strip_prefix("yes:"))
        .filter(|ssid| !ssid.is_empty())
        .unwrap_or("No conectado")
        .to_string()
}

pub fn nearby_networks(limit: usize) -> Vec<String> {
    let Some(output) = run_read_only(
        "nmcli",
        &["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
    ) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut rows = Vec::new();
    for line in output.lines() {
        let mut parts = line.split(':');
        let ssid = parts.next().unwrap_or("").trim();
        if ssid.is_empty() || !seen.insert(ssid.to_string()) {
            continue;
        }
        let signal = parts.next().unwrap_or("?").trim();
        let sec = parts.next().unwrap_or("").trim();
        let lock = if sec.is_empty() { "abierta" } else { "segura" };
        rows.push(format!("{ssid} · {signal}% · {lock}"));
        if rows.len() >= limit {
            break;
        }
    }
    rows
}

pub fn open_wifi_menu() -> Result<(), String> {
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-wifi-menu") {
        return super::run_command_status(&bin.to_string_lossy(), &[]).map(|_| ());
    }
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-wifi-menu-window") {
        return super::spawn_detached(&bin.to_string_lossy(), &[]);
    }
    Err("No se encontró nodalix-wifi-menu".to_string())
}
