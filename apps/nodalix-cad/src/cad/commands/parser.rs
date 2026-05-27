/// Parses a raw command line into tokens (command name + arguments).
pub fn parse_command_line(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|part| part.to_string())
        .collect()
}

/// Resolves aliases to canonical command names (uppercase).
pub fn resolve_command_name(token: &str) -> String {
    let upper = token.trim().to_ascii_uppercase();
    match upper.as_str() {
        "L" => "LINE".to_string(),
        "PL" | "PLINE" | "POLYLINE" => "PLINE".to_string(),
        "REC" | "RECT" | "RECTANGLE" | "R" => "RECTANGLE".to_string(),
        "C" | "CI" => "CIRCLE".to_string(),
        "M" | "MO" => "MOVE".to_string(),
        "E" | "DEL" | "ERASE" => "DELETE".to_string(),
        "U" => "UNDO".to_string(),
        "RED" | "REDO" => "REDO".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_with_points() {
        let parts = parse_command_line("line 0,0 10,5");
        assert_eq!(parts, vec!["line", "0,0", "10,5"]);
    }

    #[test]
    fn alias_resolution() {
        assert_eq!(resolve_command_name("l"), "LINE");
        assert_eq!(resolve_command_name("rec"), "RECTANGLE");
        assert_eq!(resolve_command_name("e"), "DELETE");
    }
}
