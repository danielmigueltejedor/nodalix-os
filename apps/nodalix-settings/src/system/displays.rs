use crate::system::run_read_only;

pub fn monitor_summary() -> String {
    let Some(output) = run_read_only("hyprctl", &["monitors", "-j"]) else {
        return "hyprctl no disponible".to_string();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) else {
        return "No se pudo leer la información de pantallas".to_string();
    };
    let Some(monitors) = value.as_array() else {
        return "Sin pantallas detectadas".to_string();
    };
    if monitors.is_empty() {
        return "Sin pantallas detectadas".to_string();
    }
    monitors
        .iter()
        .map(|monitor| {
            let name = monitor["name"].as_str().unwrap_or("Pantalla");
            let width = monitor["width"].as_i64().unwrap_or_default();
            let height = monitor["height"].as_i64().unwrap_or_default();
            let refresh = monitor["refreshRate"].as_f64().unwrap_or_default();
            let scale = monitor["scale"].as_f64().unwrap_or(1.0);
            format!("{name}: {width}x{height} @ {refresh:.0} Hz, escala {scale:.2}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
