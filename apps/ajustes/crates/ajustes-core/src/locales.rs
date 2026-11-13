//! The locales installed on the system (`locale -a`), with names a person can read.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locale {
    /// As GNOME stores it: `es_ES.UTF-8`.
    pub code: String,
    /// In the language itself: `Español (España)`.
    pub label: String,
}

const LANGUAGES: &[(&str, &str)] = &[
    ("ar", "العربية"),
    ("ca", "Català"),
    ("cs", "Čeština"),
    ("da", "Dansk"),
    ("de", "Deutsch"),
    ("el", "Ελληνικά"),
    ("en", "English"),
    ("es", "Español"),
    ("eu", "Euskara"),
    ("fi", "Suomi"),
    ("fr", "Français"),
    ("gl", "Galego"),
    ("he", "עברית"),
    ("hi", "हिन्दी"),
    ("hu", "Magyar"),
    ("id", "Bahasa Indonesia"),
    ("it", "Italiano"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("nb", "Norsk bokmål"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("pt", "Português"),
    ("ro", "Română"),
    ("ru", "Русский"),
    ("sv", "Svenska"),
    ("tr", "Türkçe"),
    ("uk", "Українська"),
    ("zh", "中文"),
];

const COUNTRIES: &[(&str, &str)] = &[
    ("AR", "Argentina"),
    ("AT", "Österreich"),
    ("AU", "Australia"),
    ("BE", "Belgique"),
    ("BR", "Brasil"),
    ("CA", "Canada"),
    ("CH", "Schweiz"),
    ("CL", "Chile"),
    ("CN", "中国"),
    ("CO", "Colombia"),
    ("DE", "Deutschland"),
    ("ES", "España"),
    ("FR", "France"),
    ("GB", "United Kingdom"),
    ("IE", "Ireland"),
    ("IN", "India"),
    ("IT", "Italia"),
    ("JP", "日本"),
    ("KR", "대한민국"),
    ("MX", "México"),
    ("NL", "Nederland"),
    ("PE", "Perú"),
    ("PT", "Portugal"),
    ("TW", "台灣"),
    ("US", "United States"),
    ("UY", "Uruguay"),
    ("VE", "Venezuela"),
];

fn lookup(table: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// `es_ES.utf8` → `es_ES.UTF-8`, or `None` for what is not a language locale (`C`, `POSIX`) or
/// is not UTF-8 (the desktop works in UTF-8 only).
pub fn normalize(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let (base, encoding) = raw.split_once('.')?;
    let encoding = encoding.split('@').next()?;
    if !encoding.eq_ignore_ascii_case("utf8") && !encoding.eq_ignore_ascii_case("utf-8") {
        return None;
    }
    let (lang, country) = base.split_once('_')?;
    if lang.len() < 2 || country.len() != 2 {
        return None;
    }
    Some(format!("{base}.UTF-8"))
}

/// A readable name: `Español (España)`; an unknown language or country keeps its code.
pub fn label(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let (lang, country) = base.split_once('_').unwrap_or((base, ""));
    let lang = lookup(LANGUAGES, lang).unwrap_or(lang);
    if country.is_empty() {
        return lang.to_string();
    }
    let country = lookup(COUNTRIES, country).unwrap_or(country);
    format!("{lang} ({country})")
}

/// The locales in `locale -a` output, readable, without repeats, sorted by name.
pub fn parse(output: &str) -> Vec<Locale> {
    let mut out: Vec<Locale> = Vec::new();
    for line in output.lines() {
        let Some(code) = normalize(line) else {
            continue;
        };
        if out.iter().any(|l| l.code == code) {
            continue;
        }
        let label = label(&code);
        out.push(Locale { code, label });
    }
    out.sort_by_key(|l| l.label.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_are_written_the_way_gnome_stores_them() {
        assert_eq!(normalize("es_ES.utf8").as_deref(), Some("es_ES.UTF-8"));
        assert_eq!(normalize("en_US.UTF-8").as_deref(), Some("en_US.UTF-8"));
    }

    #[test]
    fn what_is_not_a_utf8_language_locale_is_dropped() {
        for raw in ["C", "POSIX", "C.utf8", "es_ES", "es_ES.ISO-8859-1", ""] {
            assert_eq!(normalize(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_label_uses_the_language_and_country_names() {
        assert_eq!(label("es_ES.UTF-8"), "Español (España)");
        assert_eq!(label("xx_YY.UTF-8"), "xx (YY)");
    }

    #[test]
    fn the_list_has_no_repeats_and_is_sorted() {
        let list = parse("C\nPOSIX\nes_ES.utf8\nen_US.utf8\nes_ES.UTF-8\nde_DE.ISO-8859-1\n");
        let codes: Vec<_> = list.iter().map(|l| l.code.as_str()).collect();
        assert_eq!(codes, ["en_US.UTF-8", "es_ES.UTF-8"]);
    }
}
