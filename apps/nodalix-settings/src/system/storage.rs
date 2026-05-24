use crate::system::run_read_only;

pub fn filesystem_summary() -> String {
    run_read_only("df", &["-h", "-x", "tmpfs", "-x", "devtmpfs"])
        .unwrap_or_else(|| "No disponible".to_string())
}
