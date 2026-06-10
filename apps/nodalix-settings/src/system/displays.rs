use super::{run_command_status, run_read_only};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct MonitorMode {
    pub width: i64,
    pub height: i64,
    pub refresh: f64,
}

impl MonitorMode {
    pub fn label(&self) -> String {
        format!("{}×{} @ {:.0} Hz", self.width, self.height, self.refresh)
    }

    pub fn hypr_spec(&self) -> String {
        format!("{}x{}@{:.2}", self.width, self.height, self.refresh)
    }
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub description: String,
    pub width: i64,
    pub height: i64,
    pub refresh: f64,
    pub x: i64,
    pub y: i64,
    pub scale: f64,
    pub modes: Vec<MonitorMode>,
}

pub fn list_monitors() -> Result<Vec<Monitor>, String> {
    let Some(output) = run_read_only("hyprctl", &["monitors", "-j"]) else {
        return Err("hyprctl no disponible (¿estás en Hyprland?)".to_string());
    };
    let value: Value =
        serde_json::from_str(&output).map_err(|e| format!("No se pudo leer monitores: {e}"))?;
    let Some(arr) = value.as_array() else {
        return Ok(Vec::new());
    };
    Ok(arr.iter().filter_map(parse_monitor).collect())
}

fn parse_monitor(v: &Value) -> Option<Monitor> {
    let name = v["name"].as_str()?.to_string();
    let description = v["description"].as_str().unwrap_or(&name).to_string();
    let width = v["width"].as_i64().unwrap_or(0);
    let height = v["height"].as_i64().unwrap_or(0);
    let refresh = v["refreshRate"].as_f64().unwrap_or(60.0);
    let x = v["x"].as_i64().unwrap_or(0);
    let y = v["y"].as_i64().unwrap_or(0);
    let scale = v["scale"].as_f64().unwrap_or(1.0);
    let mut modes = Vec::new();
    if let Some(modes_arr) = v["availableModes"].as_array() {
        for m in modes_arr {
            if let Some(mode) = parse_mode(m) {
                modes.push(mode);
            }
        }
    }
    if modes.is_empty() {
        modes.push(MonitorMode {
            width,
            height,
            refresh,
        });
    }

    Some(Monitor {
        name,
        description,
        width,
        height,
        refresh,
        x,
        y,
        scale,
        modes,
    })
}

fn parse_mode(v: &Value) -> Option<MonitorMode> {
    Some(MonitorMode {
        width: v["width"].as_i64()?,
        height: v["height"].as_i64()?,
        refresh: v["refreshRate"].as_f64().unwrap_or(60.0),
    })
}

pub fn apply_monitor(
    name: &str,
    mode: &MonitorMode,
    x: i64,
    y: i64,
    scale: f64,
) -> Result<(), String> {
    let spec = format!("{},{},{}x{},{}", name, mode.hypr_spec(), x, y, scale);
    hypr_monitor_rule(&spec)
}

/// Hyprland ≥0.40 requires `exec` for runtime monitor changes (not bare `keyword`).
fn hypr_monitor_rule(spec: &str) -> Result<(), String> {
    let exec_cmd = format!("keyword monitor {spec}");
    if run_command_status("hyprctl", &["exec", &exec_cmd]).is_ok() {
        return Ok(());
    }
    run_command_status("hyprctl", &["keyword", "monitor", spec])
        .map_err(|first| format!("{first}. Prueba: hyprctl exec \"keyword monitor {spec}\""))
}

pub fn apply_layout(layout: &[(String, i64, i64)]) -> Result<(), String> {
    let monitors = list_monitors()?;
    for (name, x, y) in layout {
        let monitor = monitors
            .iter()
            .find(|m| &m.name == name)
            .ok_or_else(|| format!("Monitor {name} no encontrado"))?;
        let mode = MonitorMode {
            width: monitor.width,
            height: monitor.height,
            refresh: monitor.refresh,
        };
        apply_monitor(name, &mode, *x, *y, monitor.scale)?;
    }
    Ok(())
}
