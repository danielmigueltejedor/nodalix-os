use super::{run_command_status, run_read_only};

#[derive(Debug, Clone)]
pub struct BluetoothStatus {
    pub powered: bool,
    pub connected_devices: Vec<String>,
}

pub fn status() -> Result<BluetoothStatus, String> {
    let powered = bluetooth_powered().ok_or_else(|| "bluetoothctl no disponible".to_string())?;
    Ok(BluetoothStatus {
        powered,
        connected_devices: connected_devices(),
    })
}

pub fn bluetooth_powered() -> Option<bool> {
    let output = run_read_only("bluetoothctl", &["show"])?;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Powered:") {
            return Some(rest.trim().eq_ignore_ascii_case("yes"));
        }
    }
    None
}

pub fn set_bluetooth_power(on: bool) -> Result<(), String> {
    let state = if on { "on" } else { "off" };
    run_command_status("bluetoothctl", &["power", state]).map(|_| ())
}

pub fn connected_devices() -> Vec<String> {
    let Some(output) = run_read_only("bluetoothctl", &["devices", "Connected"]) else {
        return Vec::new();
    };
    output
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("Device ")?;
            let name = rest.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            if name.is_empty() { None } else { Some(name) }
        })
        .collect()
}

pub fn open_bt_menu() -> Result<(), String> {
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-bt-menu") {
        return super::spawn_detached(&bin.to_string_lossy(), &[]);
    }
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-bt-menu-window") {
        return super::spawn_detached(&bin.to_string_lossy(), &[]);
    }
    Err("No se encontró nodalix-bt-menu".to_string())
}
