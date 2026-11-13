//! Appearance: colour scheme, accent, text size and animations, all GNOME settings the shell
//! and the apps already read.

use std::{cell::Cell, rc::Rc};

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::themes::{self, Theme};

use super::{backgrounds, column, gs, scroller};
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

/// How long a click is left to be drawn before the setting it asks for is written. Writing a
/// style setting makes every app (and the shell) restyle at once, which can hold a frame for a
/// while, so the control is marked first and the system follows in the background.
const MARK_FIRST: std::time::Duration = std::time::Duration::from_millis(60);

/// A setting written a moment after the click, once the click is on screen. Clicks made in the
/// meantime replace the pending value, so only the last one is written.
#[derive(Clone, Default)]
struct Later {
    wanted: Rc<Cell<Option<&'static str>>>,
}

impl Later {
    /// Ask for `value`; it is written after the control has been drawn as chosen.
    fn set(&self, settings: &gio::Settings, key: &'static str, value: &'static str) {
        let queued = self.wanted.replace(Some(value)).is_some();
        if queued {
            return;
        }
        let (settings, wanted) = (settings.clone(), self.wanted.clone());
        glib::timeout_add_local_once(MARK_FIRST, move || {
            if let Some(value) = wanted.take() {
                if settings.string(key) != value {
                    let _ = settings.set_string(key, value);
                }
            }
        });
    }

    /// A write is on its way: the control already shows what it will be.
    fn pending(&self) -> bool {
        self.wanted.get().is_some()
    }
}

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

/// The accent palette GNOME uses (so a swatch matches what the desktop shows) and the colours of
/// the style previews, which must look light or dark whatever the app's own theme is. These are
/// data, not part of NDL's theme, so they are added here and not in `style.css`.
const ACCENT_COLORS: [(&str, &str); 9] = [
    ("blue", "#3584e4"),
    ("teal", "#2190a4"),
    ("green", "#3a944a"),
    ("yellow", "#c88800"),
    ("orange", "#ed5b00"),
    ("red", "#e62d42"),
    ("pink", "#d56199"),
    ("purple", "#9141ac"),
    ("slate", "#6f8396"),
];

fn palette_css() -> String {
    let mut css = String::new();
    for (name, color) in ACCENT_COLORS {
        css.push_str(&format!(
            ".settings-accent-{name} {{ background-color: {color}; }}\n"
        ));
    }
    css.push_str(
        ".settings-preview-prefer-light { background-image: linear-gradient(to bottom, #ffffff 0 18%, #f2f2f2 18% 100%); }\n\
         .settings-preview-prefer-dark { background-image: linear-gradient(to bottom, #3a3a3e 0 18%, #242428 18% 100%); }\n\
         .settings-preview-default { background-image: linear-gradient(105deg, #ffffff 0 18%, #f2f2f2 18% 50%, #242428 50% 100%); }\n",
    );
    css
}

/// Loads the palette once for the display.
fn load_palette() {
    thread_local! {
        static LOADED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if LOADED.with(|l| l.replace(true)) {
        return;
    }
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&palette_css());
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// The three style choices as preview cards, like GNOME's: what each would look like.
fn style_cards(settings: &gio::Settings) -> gtk::Box {
    let cards = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    cards.set_halign(gtk::Align::Center);
    let buttons: Vec<gtk::ToggleButton> = SCHEMES
        .iter()
        .map(|(label, value)| {
            let preview = gtk::Box::new(gtk::Orientation::Vertical, 0);
            preview.set_size_request(120, 78);
            preview.add_css_class("settings-preview");
            preview.add_css_class(&format!("settings-preview-{value}"));
            let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
            card.append(&preview);
            card.append(&gtk::Label::new(Some(tr(label))));
            let button = gtk::ToggleButton::new();
            button.set_child(Some(&card));
            button.add_css_class("flat");
            button.add_css_class("settings-card");
            button
        })
        .collect();
    for button in &buttons[1..] {
        button.set_group(Some(&buttons[0]));
    }
    let current =
        |settings: &gio::Settings| index_of(&SCHEMES, &settings.string("color-scheme")) as usize;
    buttons[current(settings)].set_active(true);
    let later = Later::default();
    for (button, (_, value)) in buttons.iter().zip(SCHEMES) {
        let (settings, later) = (settings.clone(), later.clone());
        button.connect_toggled(move |b| {
            if b.is_active() {
                later.set(&settings, "color-scheme", value);
            }
        });
        cards.append(button);
    }
    {
        let buttons = buttons.clone();
        settings.connect_changed(Some("color-scheme"), move |settings, _| {
            if later.pending() {
                return;
            }
            let wanted = current(settings);
            if !buttons[wanted].is_active() {
                buttons[wanted].set_active(true);
            }
        });
    }
    cards.set_sensitive(settings.is_writable("color-scheme"));
    cards
}

/// The accent colours as round swatches; the one in use shows a check.
fn accent_swatches(settings: &gio::Settings) -> gtk::Box {
    let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    swatches.set_halign(gtk::Align::Center);
    let buttons: Vec<gtk::ToggleButton> = ACCENTS
        .iter()
        .map(|(label, value)| {
            let check = gtk::Image::from_icon_name("object-select-symbolic");
            check.add_css_class("settings-swatch-check");
            let button = gtk::ToggleButton::new();
            button.set_child(Some(&check));
            button.set_tooltip_text(Some(tr(label)));
            button.add_css_class("settings-swatch");
            button.add_css_class(&format!("settings-accent-{value}"));
            button
        })
        .collect();
    for button in &buttons[1..] {
        button.set_group(Some(&buttons[0]));
    }
    let current =
        |settings: &gio::Settings| index_of(&ACCENTS, &settings.string("accent-color")) as usize;
    buttons[current(settings)].set_active(true);
    let later = Later::default();
    for (button, (_, value)) in buttons.iter().zip(ACCENTS) {
        let (settings, later) = (settings.clone(), later.clone());
        button.connect_toggled(move |b| {
            if b.is_active() {
                later.set(&settings, "accent-color", value);
            }
        });
        swatches.append(button);
    }
    {
        let buttons = buttons.clone();
        settings.connect_changed(Some("accent-color"), move |settings, _| {
            if later.pending() {
                return;
            }
            let wanted = current(settings);
            if !buttons[wanted].is_active() {
                buttons[wanted].set_active(true);
            }
        });
    }
    swatches.set_sensitive(settings.is_writable("accent-color"));
    swatches
}

/// The themes in the system and user icon folders.
fn installed_themes() -> Vec<Theme> {
    let mut roots = vec![
        glib::user_data_dir().join("icons"),
        glib::home_dir().join(".icons"),
        std::path::PathBuf::from("/usr/local/share/icons"),
        std::path::PathBuf::from("/usr/share/icons"),
    ];
    for dir in glib::system_data_dirs() {
        roots.push(dir.join("icons"));
    }
    let mut out = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(index) = std::fs::read_to_string(path.join("index.theme")) else {
                continue;
            };
            let Some(id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(theme) = themes::classify(id, &index, path.join("cursors").is_dir()) {
                out.push(theme);
            }
        }
    }
    out
}

/// A list of installed themes for a key that stores a theme's folder name.
fn theme_row(
    settings: &gio::Settings,
    key: &'static str,
    title: &str,
    icon: &str,
    list: Vec<Theme>,
) -> Option<ndl::ComboRow> {
    if !has_key(settings, key) {
        return None;
    }
    let mut ids: Vec<String> = list.iter().map(|t| t.id.clone()).collect();
    let mut names: Vec<String> = list.iter().map(|t| t.name.clone()).collect();
    let current = settings.string(key).to_string();
    // The theme in use is shown even when it was not found in the folders.
    if !ids.contains(&current) {
        ids.push(current.clone());
        names.push(current.clone());
    }
    let labels: Vec<&str> = names.iter().map(String::as_str).collect();
    let row = ndl::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&labels))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    row.set_selected(ids.iter().position(|i| *i == current).unwrap_or(0) as u32);
    row.set_sensitive(settings.is_writable(key));
    {
        let (settings, ids) = (settings.clone(), ids.clone());
        row.connect_selected_notify(move |row| {
            if let Some(id) = ids.get(row.selected() as usize) {
                if settings.string(key) != id.as_str() {
                    let _ = settings.set_string(key, id);
                }
            }
        });
    }
    {
        let row = row.clone();
        settings.connect_changed(Some(key), move |settings, key| {
            let now = settings.string(key);
            if let Some(pos) = ids.iter().position(|i| i.as_str() == now) {
                if row.selected() != pos as u32 {
                    row.set_selected(pos as u32);
                }
            }
        });
    }
    Some(row)
}

/// A font chosen with the system font dialog.
fn font_row(
    settings: &gio::Settings,
    key: &'static str,
    title: &str,
    icon: &str,
) -> Option<ndl::ActionRow> {
    if !has_key(settings, key) {
        return None;
    }
    let row = ndl::ActionRow::builder().title(title).build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    let button = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
    button.set_valign(gtk::Align::Center);
    button.set_font_desc(&gtk::pango::FontDescription::from_string(
        &settings.string(key),
    ));
    button.set_sensitive(settings.is_writable(key));
    {
        let settings = settings.clone();
        button.connect_font_desc_notify(move |b| {
            if let Some(desc) = b.font_desc() {
                let text = desc.to_str().to_string();
                if settings.string(key) != text.as_str() {
                    let _ = settings.set_string(key, &text);
                }
            }
        });
    }
    row.add_suffix(&button);
    Some(row)
}

fn titled(title: &str, rows: Vec<Option<gtk::Widget>>) -> Option<ndl::PreferencesGroup> {
    gs::group(title, rows)
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    if !schema_installed() {
        content.append(&gtk::Label::new(Some(tr("Unknown"))));
        return scroller(&content);
    }
    let settings = gio::Settings::new(SCHEMA);
    load_palette();

    // Style: how the system looks.
    let heading = ndl::PreferencesGroup::builder().title(tr("Style")).build();
    heading.add(&style_cards(&settings));
    content.append(&heading);
    if has_key(&settings, "accent-color") {
        let accent = ndl::PreferencesGroup::builder()
            .title(tr("Accent color"))
            .build();
        accent.add(&accent_swatches(&settings));
        content.append(&accent);
    }

    // Backgrounds: still pictures, the lock screen and animated ones.
    content.append(&backgrounds::section(toasts));

    // The pieces the desktop is made of.
    let all = installed_themes();
    let icons = theme_row(
        &settings,
        "icon-theme",
        tr("Icons"),
        "applications-graphics-symbolic",
        themes::sorted(all.clone(), false),
    );
    let cursor = theme_row(
        &settings,
        "cursor-theme",
        tr("Cursor"),
        "input-mouse-symbolic",
        themes::sorted(all, true),
    );
    let g = gs::open(SCHEMA);
    let cursor_size = g.as_ref().and_then(|g| {
        g.spin("cursor-size", tr("Cursor size"), (16.0, 64.0, 8.0))
            .inspect(|r| {
                r.add_prefix(&gtk::Image::from_icon_name("zoom-in-symbolic"));
            })
    });
    if let Some(group) = titled(tr("Desktop"), crate::rows![icons, cursor, cursor_size]) {
        content.append(&group);
    }

    // Text.
    let scale = ndl::SpinRow::with_range(0.5, 3.0, 0.05);
    scale.set_title(tr("Text size"));
    scale.add_prefix(&gtk::Image::from_icon_name("format-text-larger-symbolic"));
    settings
        .bind("text-scaling-factor", &scale, "value")
        .build();
    if let Some(group) = titled(
        tr("Text"),
        crate::rows![
            font_row(
                &settings,
                "font-name",
                tr("Interface font"),
                "font-x-generic-symbolic"
            ),
            font_row(
                &settings,
                "document-font-name",
                tr("Document font"),
                "x-office-document-symbolic"
            ),
            font_row(
                &settings,
                "monospace-font-name",
                tr("Monospace font"),
                "utilities-terminal-symbolic"
            ),
            Some(scale),
        ],
    ) {
        content.append(&group);
    }

    // Motion.
    let animations = ndl::SwitchRow::builder()
        .title(tr("Play animations in the interface"))
        .build();
    animations.add_prefix(&gtk::Image::from_icon_name("media-playback-start-symbolic"));
    settings
        .bind("enable-animations", &animations, "active")
        .build();
    if let Some(group) = titled(tr("Motion"), crate::rows![Some(animations)]) {
        content.append(&group);
    }

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
    fn every_accent_has_a_colour_and_the_palette_names_them_all() {
        for (_, value) in ACCENTS {
            assert!(ACCENT_COLORS.iter().any(|(n, _)| *n == value), "{value}");
        }
        assert!(palette_css().contains(".settings-accent-purple"));
    }

    #[test]
    fn every_label_is_translated() {
        // A label without a translation would show in English on a Spanish desktop.
        for (label, _) in SCHEMES.iter().chain(ACCENTS.iter()) {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
