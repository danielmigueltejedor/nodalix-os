use std::process::Command;

pub fn date_label() -> String {
    let weekday = date("+%u")
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(spanish_weekday)
        .unwrap_or("--");
    let day = date("+%d").unwrap_or_else(|| "--".to_string());
    format!("{weekday} {day}")
}

pub fn time_label() -> String {
    date("+%H:%M").unwrap_or_else(|| "--:--".to_string())
}

pub fn weather_label() -> String {
    super::capture("nodalix-weather-module", &[])
        .and_then(|text| json_text(&text))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| " --".to_string())
}

pub fn media_label() -> Option<String> {
    super::capture("nodalix-media-module", &[])
        .and_then(|text| json_text(&text))
        .map(|text| text.trim().trim_start_matches('|').trim().to_string())
        .map(|text| truncate(&text, 38))
        .filter(|text| !text.is_empty())
}

fn date(format: &str) -> Option<String> {
    Command::new("date")
        .arg(format)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn json_text(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("text")?
        .as_str()
        .map(str::to_string)
}

fn spanish_weekday(day: usize) -> Option<&'static str> {
    match day {
        1 => Some("Lun"),
        2 => Some("Mar"),
        3 => Some("Mié"),
        4 => Some("Jue"),
        5 => Some("Vie"),
        6 => Some("Sáb"),
        7 => Some("Dom"),
        _ => None,
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", shortened.trim_end())
    } else {
        text.to_string()
    }
}
