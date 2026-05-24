use crate::system::{pending_label, run_read_only};

pub fn bluetooth_status() -> String {
    pending_label(run_read_only("systemctl", &["is-active", "bluetooth"]))
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

pub fn ip_summary() -> String {
    run_read_only("hostname", &["-I"]).unwrap_or_else(|| "No disponible".to_string())
}

pub fn link_summary() -> String {
    run_read_only("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        .unwrap_or_else(|| "NetworkManager no disponible".to_string())
}
