use std::process::Command;

pub fn running_classes() -> Vec<String> {
    Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .map(|text| extract_classes(&text))
        .unwrap_or_default()
}

fn extract_classes(text: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find("\"class\":") {
        rest = &rest[index + "\"class\":".len()..];
        let trimmed = rest.trim_start();
        let Some(value) = trimmed.strip_prefix('"') else {
            continue;
        };
        let Some(end) = value.find('"') else {
            break;
        };
        classes.push(value[..end].to_lowercase());
        rest = &value[end..];
    }
    classes
}

pub fn is_running(classes: &[String], hint: &str) -> bool {
    let hint = hint.to_lowercase();
    classes.iter().any(|class| class.contains(&hint))
}
