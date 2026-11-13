//! The rules for the accounts Settings creates and changes through AccountsService.

/// The kind of account, as AccountsService numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountKind {
    Standard,
    Administrator,
}

impl AccountKind {
    pub fn from_number(n: i32) -> Self {
        if n == 1 {
            AccountKind::Administrator
        } else {
            AccountKind::Standard
        }
    }

    pub fn number(self) -> i32 {
        match self {
            AccountKind::Standard => 0,
            AccountKind::Administrator => 1,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            AccountKind::Standard => "Standard",
            AccountKind::Administrator => "Administrator",
        }
    }

    pub fn other(self) -> Self {
        match self {
            AccountKind::Standard => AccountKind::Administrator,
            AccountKind::Administrator => AccountKind::Standard,
        }
    }
}

/// Why a new account's details were turned down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NoUserName,
    BadUserName,
    BadFullName,
    NoPassword,
    PasswordsDiffer,
}

/// A user name as `useradd` takes it: lowercase letters, digits, `_` and `-`, starting with a
/// letter or `_`, at most 32 characters.
pub fn valid_user_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    name.len() <= 32
        && (first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// A full name goes into a `:`-separated file, so it cannot hold `:` or line breaks.
pub fn valid_full_name(name: &str) -> bool {
    name.chars().count() <= 128 && !name.chars().any(|c| c == ':' || c.is_control())
}

/// Checks the form for a new account.
pub fn check_new(
    user_name: &str,
    full_name: &str,
    password: &str,
    again: &str,
) -> Result<(), Problem> {
    if user_name.is_empty() {
        return Err(Problem::NoUserName);
    }
    if !valid_user_name(user_name) {
        return Err(Problem::BadUserName);
    }
    if !valid_full_name(full_name) {
        return Err(Problem::BadFullName);
    }
    check_password(password, again)
}

/// Checks a new password and its repetition.
pub fn check_password(password: &str, again: &str) -> Result<(), Problem> {
    if password.is_empty() {
        return Err(Problem::NoPassword);
    }
    if password != again {
        return Err(Problem::PasswordsDiffer);
    }
    Ok(())
}

/// A lowercase letter without its accent, for the letters Spanish and its neighbours use.
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

/// A user name suggested by a full name: lowercase letters and digits of the first word.
pub fn suggest_user_name(full_name: &str) -> String {
    let first = full_name.split_whitespace().next().unwrap_or_default();
    let mut name: String = first
        .chars()
        .filter_map(|c| {
            let c = fold(c.to_lowercase().next()?);
            (c.is_ascii_lowercase() || c.is_ascii_digit()).then_some(c)
        })
        .collect();
    while name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        name.remove(0);
    }
    name.truncate(32);
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_names_follow_useradd() {
        assert!(valid_user_name("daniel"));
        assert!(valid_user_name("_svc-1"));
        assert!(!valid_user_name(""));
        assert!(!valid_user_name("Daniel"));
        assert!(!valid_user_name("1daniel"));
        assert!(!valid_user_name("a b"));
        assert!(!valid_user_name("-a"));
        assert!(!valid_user_name(&"a".repeat(33)));
        assert!(valid_user_name(&"a".repeat(32)));
    }

    #[test]
    fn full_names_keep_the_passwd_file_intact() {
        assert!(valid_full_name("Daniel Tejedor"));
        assert!(valid_full_name(""));
        assert!(!valid_full_name("a:b"));
        assert!(!valid_full_name("a\nb"));
    }

    #[test]
    fn the_form_reports_the_first_problem() {
        assert_eq!(check_new("", "A", "x", "x"), Err(Problem::NoUserName));
        assert_eq!(check_new("A", "A", "x", "x"), Err(Problem::BadUserName));
        assert_eq!(check_new("a", "a:b", "x", "x"), Err(Problem::BadFullName));
        assert_eq!(check_new("a", "A", "", ""), Err(Problem::NoPassword));
        assert_eq!(check_new("a", "A", "x", "y"), Err(Problem::PasswordsDiffer));
        assert_eq!(check_new("a", "A", "x", "x"), Ok(()));
    }

    #[test]
    fn the_account_kinds_round_trip() {
        assert_eq!(AccountKind::from_number(1), AccountKind::Administrator);
        assert_eq!(AccountKind::from_number(0), AccountKind::Standard);
        assert_eq!(AccountKind::from_number(9), AccountKind::Standard);
        assert_eq!(AccountKind::Administrator.number(), 1);
        assert_eq!(AccountKind::Standard.other(), AccountKind::Administrator);
    }

    #[test]
    fn a_user_name_is_suggested_from_the_full_name() {
        assert_eq!(suggest_user_name("María José"), "maria");
        assert_eq!(suggest_user_name("Ñoño"), "nono");
        assert_eq!(suggest_user_name("Ana López"), "ana");
        assert_eq!(suggest_user_name("  "), "");
        assert_eq!(suggest_user_name("7zip Foo"), "zip");
    }
}
