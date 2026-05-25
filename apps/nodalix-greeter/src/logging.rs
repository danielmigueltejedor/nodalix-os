use std::{
    fs::OpenOptions,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

const LOG_PATH: &str = "/tmp/nodalix-greeter.log";

pub fn log_event(message: impl AsRef<str>) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default();
    let line = format!("[{timestamp:.3}] {}\n", message.as_ref());
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
        .and_then(|mut file| file.write_all(line.as_bytes()));
    eprintln!("nodalix-greeter: {}", message.as_ref());
}
