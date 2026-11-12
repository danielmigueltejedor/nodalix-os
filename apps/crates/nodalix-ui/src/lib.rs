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

/// Whether the user's language is Spanish, read from the usual environment variables.
pub fn spanish() -> bool {
    static ES: OnceLock<bool> = OnceLock::new();
    *ES.get_or_init(|| {
        let from = |vars: &[&str]| {
            vars.iter()
                .filter_map(|k| std::env::var(k).ok())
                .find(|v| !v.is_empty())
        };
        language_is_spanish(from(&["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]).as_deref())
    })
}

fn language_is_spanish(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.starts_with("es"))
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
        assert!(language_is_spanish(Some("es_ES.UTF-8")));
        assert!(language_is_spanish(Some("es")));
        assert!(!language_is_spanish(Some("en_US.UTF-8")));
        assert!(!language_is_spanish(None));
    }

    #[test]
    fn an_untranslated_string_comes_back_as_it_is() {
        assert_eq!(translate("Hello", &[("Other", "Otro")]), "Hello");
    }
}
