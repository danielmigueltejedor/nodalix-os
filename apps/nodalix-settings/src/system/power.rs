use crate::system::run_read_only;

pub fn active_profile() -> String {
    run_read_only("powerprofilesctl", &["get"]).unwrap_or_else(|| "No disponible".to_string())
}
