use super::{run_command_status, run_command_stdout, run_read_only};

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub volume_percent: u32,
    pub is_default: bool,
}

pub fn list_sinks() -> Result<Vec<AudioDevice>, String> {
    list_pactl_devices(true)
}

pub fn list_sources() -> Result<Vec<AudioDevice>, String> {
    list_pactl_devices(false)
}

fn list_pactl_devices(sinks: bool) -> Result<Vec<AudioDevice>, String> {
    let sub = if sinks { "sinks" } else { "sources" };
    let out = run_command_stdout("pactl", &["list", sub, "short"])?;
    let default = if sinks {
        run_read_only("pactl", &["get-default-sink"]).unwrap_or_default()
    } else {
        run_read_only("pactl", &["get-default-source"]).unwrap_or_default()
    };

    let devices = out
        .lines()
        .filter_map(|line| {
            let mut p = line.split_whitespace();
            let id = p.next()?.to_string();
            let name = p.next()?.to_string();
            let desc = p.collect::<Vec<_>>().join(" ");
            let vol = if sinks {
                sink_volume_percent(&name)
            } else {
                50
            };
            Some(AudioDevice {
                is_default: name == default,
                id,
                name: name.clone(),
                description: if desc.is_empty() { name } else { desc },
                volume_percent: vol,
            })
        })
        .collect();
    Ok(devices)
}

pub fn set_default_sink(id_or_name: &str) -> Result<(), String> {
    if super::command_exists("wpctl") {
        return run_command_status("wpctl", &["set-default", id_or_name]).map(|_| ());
    }
    run_command_status("pactl", &["set-default-sink", id_or_name]).map(|_| ())
}

pub fn set_default_source(id_or_name: &str) -> Result<(), String> {
    if super::command_exists("wpctl") {
        return run_command_status("wpctl", &["set-default", id_or_name]).map(|_| ());
    }
    run_command_status("pactl", &["set-default-source", id_or_name]).map(|_| ())
}

pub fn set_volume(device_id: &str, percent: u32) -> Result<(), String> {
    let pct = percent.min(150);
    if super::command_exists("wpctl") {
        return run_command_status("wpctl", &["set-volume", device_id, &format!("{pct}%")])
            .map(|_| ());
    }
    run_command_status("pactl", &["set-sink-volume", device_id, &format!("{pct}%")]).map(|_| ())
}

pub fn sink_volume_percent(id: &str) -> u32 {
    if let Ok(out) = run_command_stdout("wpctl", &["get-volume", id]) {
        if let Some(pct) = out.split_whitespace().find(|t| t.ends_with('%')) {
            return pct.trim_end_matches('%').parse().unwrap_or(50);
        }
        if let Ok(v) = out.trim().parse::<f32>() {
            return (v * 100.0).round() as u32;
        }
    }
    50
}

pub fn open_audio_panel() -> Result<(), String> {
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-volume-menu-window") {
        return super::spawn_detached(&bin.to_string_lossy(), &[]);
    }
    if let Some(bin) = super::resolve_nodalix_bin("nodalix-volume-menu") {
        return super::spawn_detached(&bin.to_string_lossy(), &[]);
    }
    if super::command_exists("pavucontrol") {
        return super::spawn_detached("pavucontrol", &[]);
    }
    Err("Instala pavucontrol o nodalix-volume-menu".to_string())
}
