//! What every native Nodalix app shares.

use std::sync::OnceLock;

use ndl::gtk;
use nodalia_gtk as ndl;

const BASE: &str = include_str!("base.css");

/// Installs NDL, then Nodalix's base style, then the app's own CSS, all above NDL's sheets. Call
/// once from the application's `startup`.
pub fn init(app_css: &str) {
    ndl::init();
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    for (css, extra) in [(BASE, 40), (app_css, 50)] {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(css);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + extra,
        );
    }
}

/// Whether the user's language is Spanish, chosen the way gettext chooses it.
pub fn spanish() -> bool {
    static ES: OnceLock<bool> = OnceLock::new();
    *ES.get_or_init(|| {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let locale = var("LC_ALL")
            .or_else(|| var("LC_MESSAGES"))
            .or_else(|| var("LANG"));
        language_is_spanish(var("LANGUAGE").as_deref(), locale.as_deref())
    })
}

/// `language` is the `LANGUAGE` preference list (`es:en`), `locale` the first of `LC_ALL`,
/// `LC_MESSAGES` and `LANG` that is set. In the C and POSIX locales translations are off and
/// `LANGUAGE` is ignored; otherwise its first entry that is English or Spanish wins over the
/// locale, which only decides when the list names neither.
fn language_is_spanish(language: Option<&str>, locale: Option<&str>) -> bool {
    if locale.is_none_or(|l| l == "C" || l == "POSIX" || l.starts_with("C.")) {
        return false;
    }
    let listed = language
        .unwrap_or("")
        .split(':')
        .find_map(|entry| match entry {
            e if e.starts_with("es") => Some(true),
            e if e.starts_with("en") => Some(false),
            _ => None,
        });
    listed.unwrap_or_else(|| locale.is_some_and(|l| l.starts_with("es")))
}

/// A tiny catalog: English is the source text and `es` lists its Spanish translations as
/// `(english, spanish)`. The OS handles locales for the rest of the desktop; an app only needs
/// its own few dozen strings.
pub fn translate(text: &'static str, es: &[(&'static str, &'static str)]) -> &'static str {
    if !spanish() {
        return text;
    }
    es.iter()
        .find(|(en, _)| *en == text)
        .map_or(text, |(_, es)| es)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_locales() {
        assert!(language_is_spanish(None, Some("es_ES.UTF-8")));
        assert!(language_is_spanish(None, Some("es")));
        assert!(!language_is_spanish(None, Some("en_US.UTF-8")));
        assert!(!language_is_spanish(None, None));
    }

    #[test]
    fn language_beats_the_locale_except_in_c() {
        // LC_MESSAGES=en_US with LANGUAGE=es: gettext picks Spanish.
        assert!(language_is_spanish(Some("es"), Some("en_US.UTF-8")));
        assert!(!language_is_spanish(Some("en:es"), Some("es_ES.UTF-8")));
        // A list that names neither falls back to the locale.
        assert!(language_is_spanish(Some("fr:de"), Some("es_ES.UTF-8")));
        assert!(language_is_spanish(Some("fr:es"), Some("en_US.UTF-8")));
        // In the C locale LANGUAGE is ignored.
        assert!(!language_is_spanish(Some("es"), Some("C")));
        assert!(!language_is_spanish(Some("es"), Some("POSIX")));
        assert!(!language_is_spanish(Some("es"), None));
    }

    #[test]
    fn an_untranslated_string_comes_back_as_it_is() {
        assert_eq!(translate("Hello", &[("Other", "Otro")]), "Hello");
    }
}
