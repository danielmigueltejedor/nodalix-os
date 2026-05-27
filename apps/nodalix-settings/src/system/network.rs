use super::{run_command_status, run_command_stdout, run_read_only, spawn_detached};

#[derive(Debug, Clone)]
pub struct NetDevice {
    pub name: String,
    pub kind: String,
    pub state: String,
    pub connection: String,
}

#[derive(Debug, Clone)]
pub struct NetConnection {
    pub name: String,
    pub _uuid: String,
    pub kind: String,
    pub device: String,
    pub active: bool,
}

pub fn list_devices() -> Result<Vec<NetDevice>, String> {
    let out = run_command_stdout("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"])?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let mut p = line.split(':');
            Some(NetDevice {
                name: p.next()?.to_string(),
                kind: p.next()?.to_string(),
                state: p.next()?.to_string(),
                connection: p.next().unwrap_or("").to_string(),
            })
        })
        .collect())
}

pub fn list_connections() -> Result<Vec<NetConnection>, String> {
    let out = run_command_stdout("nmcli", &["-t", "-f", "NAME,UUID,TYPE,DEVICE", "connection", "show"])?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let mut p = line.split(':');
            let name = p.next()?.to_string();
            let _uuid = p.next()?.to_string();
            let kind = p.next()?.to_string();
            let device = p.next().unwrap_or("").to_string();
            let active = run_read_only("nmcli", &["-t", "-f", "GENERAL.STATE", "connection", "show", &name])
                .map(|s| s.contains("activated"))
                .unwrap_or(false);
            Some(NetConnection {
                name,
                _uuid,
                kind,
                device,
                active,
            })
        })
        .collect())
}

pub fn activate_connection(name: &str) -> Result<(), String> {
    run_command_status("nmcli", &["connection", "up", name]).map(|_| ())
}

pub fn disconnect_device(device: &str) -> Result<(), String> {
    run_command_status("nmcli", &["device", "disconnect", device]).map(|_| ())
}

pub fn dns_servers() -> String {
    run_read_only("nmcli", &["-t", "-f", "IP4.DNS", "device", "show"])
        .unwrap_or_else(|| "No disponible".to_string())
}

pub fn ip_summary() -> String {
    run_read_only("hostname", &["-I"]).unwrap_or_else(|| "No disponible".to_string())
}

pub fn open_network_menu() -> Result<(), String> {
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-wifi-menu-window") {
        return spawn_detached(&bin.to_string_lossy(), &[]);
    }
    if super::command_exists("nm-connection-editor") {
        return spawn_detached("nm-connection-editor", &[]);
    }
    Err("No se encontró nm-connection-editor".to_string())
}
