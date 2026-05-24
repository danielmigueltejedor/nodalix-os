use crate::system::run_read_only;

pub fn pending_updates() -> String {
    match run_read_only("checkupdates", &[]) {
        Some(output) => output.lines().count().to_string(),
        None => "No disponible".to_string(),
    }
}
