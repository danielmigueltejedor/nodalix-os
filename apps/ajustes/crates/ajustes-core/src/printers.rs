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

/// A printer found on the network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub uri: String,
}

/// The printers `ippfind` printed as `name|uri` lines, one per printer, without repeats.
pub fn parse_found(output: &str) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for line in output.lines() {
        let Some((name, uri)) = line.split_once('|') else {
            continue;
        };
        let (name, uri) = (name.trim(), uri.trim());
        if name.is_empty() || !is_safe_uri(uri) || out.iter().any(|f| f.uri == uri) {
            continue;
        }
        out.push(Found {
            name: name.to_owned(),
            uri: uri.to_owned(),
        });
    }
    out
}

/// A device address CUPS can use and nothing that would be read as an option or a shell word.
fn is_safe_uri(uri: &str) -> bool {
    const SCHEMES: [&str; 6] = [
        "ipp://",
        "ipps://",
        "http://",
        "https://",
        "socket://",
        "lpd://",
    ];
    SCHEMES.iter().any(|s| uri.starts_with(s))
        && uri.len() <= 512
        && uri.chars().all(|c| {
            c.is_ascii_graphic()
                && !matches!(
                    c,
                    '"' | '\'' | '`' | '$' | ';' | '|' | '&' | '<' | '>' | '\\'
                )
        })
}

/// What a person typed as the address of a printer, as a URI to give CUPS: a full address is
/// kept; a bare host or `host:port` becomes an IPP address.
pub fn normalize_address(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() || input.contains(char::is_whitespace) {
        return None;
    }
    let uri = if input.contains("://") {
        input.to_owned()
    } else {
        format!("ipp://{input}/ipp/print")
    };
    is_safe_uri(&uri).then_some(uri)
}

/// A queue name for a printer title that is valid and not taken: letters, digits and `_`/`-`.
pub fn queue_name(title: &str, existing: &[String]) -> String {
    let mut base: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    base = base.trim_matches('_').to_owned();
    if base.is_empty() {
        base = "Printer".to_owned();
    }
    base.truncate(40);
    let mut name = base.clone();
    let mut n = 2;
    while existing.contains(&name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    name
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

    #[test]
    fn found_printers_are_parsed_without_repeats_or_odd_addresses() {
        let out = "HP LaserJet|ipp://hp.local:631/ipp/print\nHP LaserJet|ipp://hp.local:631/ipp/print\nBad|file:///etc/passwd\nNo bar\nEvil|ipp://x/$(reboot)\n";
        let found = parse_found(out);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "HP LaserJet");
    }

    #[test]
    fn a_host_becomes_an_ipp_address_and_a_uri_is_kept() {
        assert_eq!(
            normalize_address("192.168.1.20").as_deref(),
            Some("ipp://192.168.1.20/ipp/print")
        );
        assert_eq!(
            normalize_address("socket://10.0.0.5:9100").as_deref(),
            Some("socket://10.0.0.5:9100")
        );
        assert_eq!(normalize_address("two words"), None);
        assert_eq!(normalize_address("file:///x"), None);
        assert_eq!(normalize_address(""), None);
        assert_eq!(normalize_address("host;reboot"), None);
    }

    #[test]
    fn queue_names_are_valid_and_unique() {
        assert_eq!(queue_name("HP LaserJet Pro", &[]), "HP_LaserJet_Pro");
        assert_eq!(queue_name("   ", &[]), "Printer");
        let taken = vec!["Office".to_owned(), "Office-2".to_owned()];
        assert_eq!(queue_name("Office", &taken), "Office-3");
        assert!(is_valid_name(&queue_name("Ünï -x", &[])));
    }
}
