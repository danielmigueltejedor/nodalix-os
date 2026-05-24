pub fn label() -> String {
    if let Some(output) = super::capture("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
        return format!("󰕾 {}", output.replace("Volume: ", ""));
    }
    if let Some(output) = super::capture("pamixer", &["--get-volume-human"]) {
        return format!("󰕾 {output}");
    }
    "󰕾 --".to_string()
}
