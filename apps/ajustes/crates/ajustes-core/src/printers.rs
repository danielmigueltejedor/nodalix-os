//! Printers, as `lpstat -p` and `lpstat -d` report them. CUPS answers the same commands whatever
//! the print system behind it, so the page needs no CUPS library.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Printing,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Printer {
    pub name: String,
    pub state: State,
    pub is_default: bool,
}

impl Printer {
    /// `HP_LaserJet_Pro` → `HP LaserJet Pro`: CUPS names have no spaces.
    pub fn title(&self) -> String {
        self.name.replace('_', " ")
    }
}

/// The printers in `lpstat -p` output. A line is `printer NAME is idle.`, `printer NAME now
/// printing NAME-12.` or `printer NAME disabled since …`; anything else (a job line, a message in
/// another language) is not a printer and is skipped.
fn parse_list(output: &str) -> Vec<(String, State)> {
    let mut out = Vec::new();
    for line in output.lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("printer") {
            continue;
        }
        let Some(name) = words.next() else { continue };
        let rest: Vec<&str> = words.collect();
        let state = if rest.starts_with(&["disabled"]) {
            State::Disabled
        } else if rest.starts_with(&["now", "printing"]) {
            State::Printing
        } else if rest.starts_with(&["is", "idle."]) || rest.starts_with(&["is", "idle"]) {
            State::Idle
        } else {
            continue;
        };
        out.push((name.to_owned(), state));
    }
    out
}

/// The name in `lpstat -d` output (`system default destination: NAME`), if there is one.
fn parse_default(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|l| l.strip_prefix("system default destination:"))
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
}

pub fn parse(list: &str, default: &str) -> Vec<Printer> {
    let default = parse_default(default);
    parse_list(list)
        .into_iter()
        .map(|(name, state)| Printer {
            is_default: default.as_deref() == Some(name.as_str()),
            name,
            state,
        })
        .collect()
}

/// A destination name is safe to pass as an argument: CUPS allows no spaces, slashes or `#`, and
/// one that starts with `-` would be read as an option.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '@' | ':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "printer HP_LaserJet is idle.  enabled since Thu 12 Nov 2026 20:00:00 CET\n\
printer Brother now printing Brother-12.  enabled since Thu 12 Nov 2026 20:00:00 CET\n\
\tconnecting to printer\n\
printer Old_Epson disabled since Thu 12 Nov 2026 19:00:00 CET -\n\
\treason unknown\n";

    #[test]
    fn printers_and_their_states_are_read() {
        let list = parse(LIST, "system default destination: Brother\n");
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].state, State::Idle);
        assert_eq!(list[1].state, State::Printing);
        assert!(list[1].is_default);
        assert_eq!(list[2].state, State::Disabled);
        assert!(!list[0].is_default);
    }

    #[test]
    fn no_default_and_no_printers_are_fine() {
        assert!(parse("", "no system default destination\n").is_empty());
        let list = parse(LIST, "no system default destination\n");
        assert!(list.iter().all(|p| !p.is_default));
    }

    #[test]
    fn a_title_replaces_underscores() {
        let p = &parse(LIST, "")[0];
        assert_eq!(p.title(), "HP LaserJet");
    }

    #[test]
    fn only_safe_names_reach_a_command() {
        assert!(is_valid_name("HP_LaserJet-2"));
        assert!(!is_valid_name("-o"));
        assert!(!is_valid_name("a b"));
        assert!(!is_valid_name("a/b"));
        assert!(!is_valid_name(""));
    }
}
