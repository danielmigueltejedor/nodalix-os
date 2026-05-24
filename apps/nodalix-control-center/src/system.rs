use std::process::Command;

pub fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", shell_escape(name)))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn run_command(program: &str, args: &[&str]) -> bool {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            eprintln!(
                "nodalix-control-center: command failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            false
        }
        Err(error) => {
            eprintln!("nodalix-control-center: command error: {error}");
            false
        }
    }
}

pub fn volume_label() -> String {
    if let Some(output) = run_capture("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
        return output.replace("Volume: ", "");
    }
    if let Some(output) = run_capture("pamixer", &["--get-volume-human"]) {
        return output;
    }
    "No disponible".to_string()
}

pub fn brightness_label() -> String {
    if let Some(output) = run_capture("brightnessctl", &["-m"]) {
        let parts: Vec<&str> = output.split(',').collect();
        if let Some(percent) = parts.get(3) {
            return (*percent).to_string();
        }
    }
    "Sin dispositivo".to_string()
}

pub fn media_label() -> String {
    run_capture(
        "playerctl",
        &["metadata", "--format", "{{artist}} - {{title}}"],
    )
    .unwrap_or_else(|| "Sin reproducción activa".to_string())
}

pub fn wifi_label() -> String {
    run_capture("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .and_then(|rows| {
            rows.lines()
                .find_map(|line| line.strip_prefix("yes:").map(str::to_string))
        })
        .filter(|ssid| !ssid.is_empty())
        .unwrap_or_else(|| "Wi-Fi".to_string())
}

pub fn bluetooth_label() -> String {
    run_capture("bluetoothctl", &["show"])
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.trim().strip_prefix("Powered: ").map(str::to_string))
        })
        .map(|state| {
            if state == "yes" {
                "Bluetooth activo"
            } else {
                "Bluetooth apagado"
            }
        })
        .unwrap_or("Bluetooth")
        .to_string()
}

fn shell_escape(value: &str) -> String {
    value.replace('\'', "'\\''")
}
