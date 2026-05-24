use crate::system::run_read_only;

pub fn default_sink() -> String {
    run_read_only("wpctl", &["status"])
        .or_else(|| run_read_only("pactl", &["get-default-sink"]))
        .unwrap_or_else(|| "No disponible".to_string())
}

pub fn default_source() -> String {
    run_read_only("pactl", &["get-default-source"]).unwrap_or_else(|| "No disponible".to_string())
}
