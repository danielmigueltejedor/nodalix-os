//! Appearance: colour scheme, accent, text size and animations, all GNOME settings the shell
//! and the apps already read.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, prelude::*},
};

use super::{column, scroller};
use crate::i18n::tr;

const SCHEMA: &str = "org.gnome.desktop.interface";

/// `(label, gsettings value)` for the colour scheme.
const SCHEMES: [(&str, &str); 3] = [
    ("Follow the system", "default"),
    ("Light", "prefer-light"),
    ("Dark", "prefer-dark"),
];
const ACCENTS: [(&str, &str); 9] = [
    ("Blue", "blue"),
    ("Teal", "teal"),
    ("Green", "green"),
    ("Yellow", "yellow"),
    ("Orange", "orange"),
    ("Red", "red"),
    ("Pink", "pink"),
    ("Purple", "purple"),
    ("Slate", "slate"),
];

/// Index of `value` in a table, or 0.
pub fn index_of(table: &[(&str, &str)], value: &str) -> u32 {
    table.iter().position(|(_, v)| *v == value).unwrap_or(0) as u32
}

fn has_key(settings: &gio::Settings, key: &str) -> bool {
    settings.settings_schema().is_some_and(|s| s.has_key(key))
}

fn schema_installed() -> bool {
    gio::SettingsSchemaSource::default().is_some_and(|s| s.lookup(SCHEMA, true).is_some())
}

fn choice_row(
    settings: &gio::Settings,
    key: &'static str,
    title: &str,
    table: &'static [(&'static str, &'static str)],
) -> ndl::ComboRow {
    let labels: Vec<&str> = table.iter().map(|(label, _)| tr(label)).collect();
    let row = ndl::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&labels))
        .build();
    row.set_selected(index_of(table, &settings.string(key)));
    // A key an administrator locked cannot be changed from here.
    row.set_sensitive(settings.is_writable(key));
    {
        // Keeps the row in step with the key when something else changes it while this window
        // is open. Setting `selected` only when it differs avoids writing the value back.
        let row = row.clone();
        settings.connect_changed(Some(key), move |settings, key| {
            let wanted = index_of(table, &settings.string(key));
            if row.selected() != wanted {
                row.set_selected(wanted);
            }
        });
    }
    {
        let settings = settings.clone();
        row.connect_selected_notify(move |row| {
            let Some((_, value)) = table.get(row.selected() as usize) else {
                return;
            };
            if settings.string(key) == *value {
                return;
            }
            if settings.set_string(key, value).is_err() {
                // Refused: show what is really stored, not the choice that did not apply.
                row.set_selected(index_of(table, &settings.string(key)));
            }
        });
    }
    row
}

pub fn build() -> gtk::ScrolledWindow {
    let content = column();
    if !schema_installed() {
        content.append(&gtk::Label::new(Some(tr("Unknown"))));
        return scroller(&content);
    }
    let settings = gio::Settings::new(SCHEMA);

    let style = ndl::PreferencesGroup::builder().title(tr("Style")).build();
    style.add(&choice_row(
        &settings,
        "color-scheme",
        tr("Color scheme"),
        &SCHEMES,
    ));
    if has_key(&settings, "accent-color") {
        style.add(&choice_row(
            &settings,
            "accent-color",
            tr("Accent color"),
            &ACCENTS,
        ));
    }
    content.append(&style);

    let text = ndl::PreferencesGroup::new();
    let scale = ndl::SpinRow::with_range(0.5, 3.0, 0.05);
    scale.set_title(tr("Text size"));
    settings
        .bind("text-scaling-factor", &scale, "value")
        .build();
    text.add(&scale);
    content.append(&text);

    let motion = ndl::PreferencesGroup::new();
    let animations = ndl::SwitchRow::builder()
        .title(tr("Play animations in the interface"))
        .build();
    settings
        .bind("enable-animations", &animations, "active")
        .build();
    motion.add(&animations);
    content.append(&motion);

    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_index_of_a_value_and_falls_back_to_the_first() {
        assert_eq!(index_of(&SCHEMES, "prefer-dark"), 2);
        assert_eq!(index_of(&SCHEMES, "default"), 0);
        assert_eq!(index_of(&SCHEMES, "nonsense"), 0);
        assert_eq!(index_of(&ACCENTS, "purple"), 7);
    }

    #[test]
    fn every_label_is_translated() {
        // A label without a translation would show in English on a Spanish desktop.
        for (label, _) in SCHEMES.iter().chain(ACCENTS.iter()) {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
