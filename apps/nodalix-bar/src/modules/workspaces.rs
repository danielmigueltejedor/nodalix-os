pub fn label() -> String {
    if !super::command_exists("hyprctl") {
        return "Workspace".to_string();
    }
    super::capture("hyprctl", &["activeworkspace", "-j"])
        .and_then(|text| extract_json_string(&text, "name"))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Workspace".to_string())
}

fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
