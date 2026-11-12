//! Rows bound to GSettings keys, the way most of the GNOME settings are stored. Every helper
//! returns `None` when the schema or the key is not installed (another GNOME version, or a part
//! of the desktop that is missing), so a page shows the rows that exist and never aborts: reading
//! a key that does not exist ends the process in GIO.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use crate::i18n::tr;

/// A value a choice maps to: the key is a string (also an enum) or an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Val {
    S(&'static str),
    I(i64),
}

impl Val {
    fn of(variant: &glib::Variant) -> Option<Val> {
        // Only integers are read back as a `Val`; strings are compared in place by `matches`.
        match variant.type_().as_str() {
            "i" => variant.get::<i32>().map(|v| Val::I(v.into())),
            "u" => variant.get::<u32>().map(|v| Val::I(v.into())),
            "x" => variant.get::<i64>().map(Val::I),
            "t" => variant
                .get::<u64>()
                .and_then(|v| i64::try_from(v).ok().map(Val::I)),
            _ => None,
        }
    }

    fn matches(self, variant: &glib::Variant) -> bool {
        match self {
            Val::S(s) => variant.str() == Some(s),
            Val::I(_) => Val::of(variant) == Some(self),
        }
    }

    /// The value as a variant of the same type as the key's.
    fn to_variant(self, like: &glib::Variant) -> Option<glib::Variant> {
        match (self, like.type_().as_str()) {
            (Val::S(s), "s") => Some(s.to_variant()),
            (Val::I(v), "i") => i32::try_from(v).ok().map(|v| v.to_variant()),
            (Val::I(v), "u") => u32::try_from(v).ok().map(|v| v.to_variant()),
            (Val::I(v), "x") => Some(v.to_variant()),
            (Val::I(v), "t") => u64::try_from(v).ok().map(|v| v.to_variant()),
            _ => None,
        }
    }
}

/// The icon that stands for a key in its row, like the icons in GNOME's settings: found by what
/// the key's name talks about, so a new key of a known kind gets one without a table entry.
pub fn icon_for(key: &str) -> Option<&'static str> {
    const ICONS: &[(&str, &str)] = &[
        // More specific names first: the first that the key contains wins.
        ("locate-pointer", "input-mouse-symbolic"),
        ("double-click", "input-mouse-symbolic"),
        ("left-handed", "input-mouse-symbolic"),
        ("mousekeys", "input-mouse-symbolic"),
        ("bouncekeys", "input-keyboard-symbolic"),
        ("slowkeys", "input-keyboard-symbolic"),
        ("stickykeys", "input-keyboard-symbolic"),
        ("togglekeys", "input-keyboard-symbolic"),
        ("screen-keyboard", "input-keyboard-symbolic"),
        ("allow-volume", "audio-volume-high-symbolic"),
        ("show-status-shapes", "view-reveal-symbolic"),
        ("visual-bell", "display-brightness-symbolic"),
        ("automatic-timezone", "find-location-symbolic"),
        ("show-banners", "preferences-system-notifications-symbolic"),
        ("battery", "battery-symbolic"),
        ("power-saver", "battery-symbolic"),
        ("remember-app-usage", "view-app-grid-symbolic"),
        ("remove-old-temp", "edit-clear-all-symbolic"),
        ("remove-old-trash", "user-trash-symbolic"),
        ("night-light", "night-light-symbolic"),
        // "clock" contains "lock", so it comes first.
        ("clock", "preferences-system-time-symbolic"),
        ("lock", "system-lock-screen-symbolic"),
        ("idle-dim", "display-brightness-symbolic"),
        ("idle", "preferences-system-time-symbolic"),
        ("power-button", "system-shutdown-symbolic"),
        ("sleep", "system-suspend-symbolic"),
        ("tap", "input-touchpad-symbolic"),
        ("click", "input-touchpad-symbolic"),
        ("scroll", "input-touchpad-symbolic"),
        ("send-events", "input-touchpad-symbolic"),
        ("accel", "input-mouse-symbolic"),
        ("speed", "input-mouse-symbolic"),
        ("cursor", "input-mouse-symbolic"),
        ("typing", "input-keyboard-symbolic"),
        ("repeat", "input-keyboard-symbolic"),
        ("delay", "input-keyboard-symbolic"),
        ("numlock", "input-keyboard-symbolic"),
        ("event-sounds", "audio-volume-high-symbolic"),
        ("old-files", "user-trash-symbolic"),
        ("recent", "document-open-recent-symbolic"),
        ("report", "dialog-warning-symbolic"),
        ("usb", "media-removable-symbolic"),
        ("magnifier", "zoom-in-symbolic"),
        (
            "screen-reader",
            "preferences-desktop-accessibility-symbolic",
        ),
        ("picture", "preferences-desktop-wallpaper-symbolic"),
        ("workspace", "view-grid-symbolic"),
        ("hot-corner", "view-fullscreen-symbolic"),
        ("edge-tiling", "view-dual-symbolic"),
        ("animation", "media-playback-start-symbolic"),
        ("cursor-blink", "input-keyboard-symbolic"),
        ("high-contrast", "display-brightness-symbolic"),
        ("large-text", "format-text-larger-symbolic"),
        ("overlay-scrolling", "input-mouse-symbolic"),
    ];
    ICONS
        .iter()
        .find(|(needle, _)| key.contains(needle))
        .map(|(_, icon)| *icon)
}

pub const fn s(label: &'static str, value: &'static str) -> (&'static str, Val) {
    (label, Val::S(value))
}

pub const fn i(label: &'static str, value: i64) -> (&'static str, Val) {
    (label, Val::I(value))
}

/// The settings of one schema, if it is installed.
#[derive(Clone)]
pub struct Gs(gio::Settings);

pub fn open(schema: &str) -> Option<Gs> {
    let source = gio::SettingsSchemaSource::default()?;
    source.lookup(schema, true)?;
    Some(Gs(gio::Settings::new(schema)))
}

/// A relocatable schema at `path`.
pub fn open_path(schema: &str, path: &str) -> Option<Gs> {
    let source = gio::SettingsSchemaSource::default()?;
    source.lookup(schema, true)?;
    Some(Gs(gio::Settings::with_path(schema, path)))
}

impl Gs {
    pub fn has(&self, key: &str) -> bool {
        self.0.settings_schema().is_some_and(|s| s.has_key(key))
    }

    pub fn settings(&self) -> &gio::Settings {
        &self.0
    }

    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.has(key).then(|| self.0.boolean(key))
    }

    /// A switch for a boolean key.
    pub fn switch(&self, key: &str, title: &str, subtitle: &str) -> Option<ndl::SwitchRow> {
        if !self.has(key) {
            return None;
        }
        let row = ndl::SwitchRow::builder().title(title).build();
        if let Some(icon) = icon_for(key) {
            row.add_prefix(&gtk::Image::from_icon_name(icon));
        }
        if !subtitle.is_empty() {
            row.set_subtitle(subtitle);
        }
        self.0.bind(key, &row, "active").build();
        // A key an administrator locked cannot be changed from here.
        row.set_sensitive(self.0.is_writable(key));
        Some(row)
    }

    /// A number box for a numeric key.
    pub fn spin(
        &self,
        key: &str,
        title: &str,
        (min, max, step): (f64, f64, f64),
    ) -> Option<ndl::SpinRow> {
        if !self.has(key) {
            return None;
        }
        let row = ndl::SpinRow::with_range(min, max, step);
        row.set_title(title);
        if let Some(icon) = icon_for(key) {
            row.add_prefix(&gtk::Image::from_icon_name(icon));
        }
        self.0.bind(key, &row, "value").build();
        row.set_sensitive(self.0.is_writable(key));
        Some(row)
    }

    /// A list of choices for a string or integer key. A current value the table does not list
    /// is shown as its own entry instead of being replaced by the first choice.
    pub fn choice(
        &self,
        key: &'static str,
        title: &str,
        table: &'static [(&'static str, Val)],
    ) -> Option<ndl::ComboRow> {
        if !self.has(key) {
            return None;
        }
        let current = self.0.value(key);
        let mut labels: Vec<String> = table.iter().map(|(l, _)| tr(l).to_string()).collect();
        let unlisted = !table.iter().any(|(_, v)| v.matches(&current));
        if unlisted {
            labels.push(current.print(false).to_string());
        }
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let row = ndl::ComboRow::builder()
            .title(title)
            .model(&gtk::StringList::new(&labels))
            .build();
        if let Some(icon) = icon_for(key) {
            row.add_prefix(&gtk::Image::from_icon_name(icon));
        }
        row.set_selected(selected_for(table, &current));
        row.set_sensitive(self.0.is_writable(key));
        {
            // Keeps the row in step when something else changes the key while this window is
            // open; setting `selected` only when it differs avoids writing the value back.
            let row = row.clone();
            self.0.connect_changed(Some(key), move |settings, key| {
                let wanted = selected_for(table, &settings.value(key));
                if row.selected() != wanted && (wanted as usize) < table.len() {
                    row.set_selected(wanted);
                }
            });
        }
        {
            let settings = self.0.clone();
            row.connect_selected_notify(move |row| {
                let Some((_, value)) = table.get(row.selected() as usize) else {
                    // The entry for a value that is not in the table: nothing to write.
                    return;
                };
                let current = settings.value(key);
                if value.matches(&current) {
                    return;
                }
                let written = value
                    .to_variant(&current)
                    .is_some_and(|v| settings.set_value(key, &v).is_ok());
                if !written {
                    // Refused: show what is really stored, not the choice that did not apply.
                    row.set_selected(selected_for(table, &settings.value(key)));
                }
            });
        }
        Some(row)
    }

    /// A slider for a numeric key, in an action row.
    pub fn slider(
        &self,
        key: &str,
        title: &str,
        subtitle: &str,
        (min, max, step): (f64, f64, f64),
    ) -> Option<ndl::ActionRow> {
        if !self.has(key) {
            return None;
        }
        let row = ndl::ActionRow::builder().title(title).build();
        if let Some(icon) = icon_for(key) {
            row.add_prefix(&gtk::Image::from_icon_name(icon));
        }
        if !subtitle.is_empty() {
            row.set_subtitle(subtitle);
        }
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, step);
        scale.set_width_request(220);
        scale.set_valign(gtk::Align::Center);
        scale.set_draw_value(false);
        self.0.bind(key, &scale.adjustment(), "value").build();
        scale.set_sensitive(self.0.is_writable(key));
        row.add_suffix(&scale);
        Some(row)
    }
}

fn selected_for(table: &[(&str, Val)], current: &glib::Variant) -> u32 {
    table
        .iter()
        .position(|(_, v)| v.matches(current))
        .unwrap_or(table.len()) as u32
}

/// Something that can be a row of a group: a widget, or one that may not exist on this system.
pub trait Row {
    fn into_row(self) -> Option<gtk::Widget>;
}

impl<T: IsA<gtk::Widget>> Row for Option<T> {
    fn into_row(self) -> Option<gtk::Widget> {
        self.map(|w| w.upcast())
    }
}

/// The rows of a group, each of them a widget or an `Option` of one.
#[macro_export]
macro_rules! rows {
    ($($row:expr),* $(,)?) => {
        vec![$($crate::pages::gs::Row::into_row($row)),*]
    };
}

/// Adds every row that exists; a group without any is not added.
pub fn group(title: &str, rows: Vec<Option<gtk::Widget>>) -> Option<ndl::PreferencesGroup> {
    let rows: Vec<_> = rows.into_iter().flatten().collect();
    if rows.is_empty() {
        return None;
    }
    let group = ndl::PreferencesGroup::new();
    if !title.is_empty() {
        group.set_title(title);
    }
    for row in &rows {
        group.add(row);
    }
    Some(group)
}

/// A page's worth of groups in a scroller; says so when nothing could be shown.
pub fn page(groups: Vec<Option<ndl::PreferencesGroup>>) -> gtk::ScrolledWindow {
    let content = super::column();
    let mut any = false;
    for g in groups.into_iter().flatten() {
        content.append(&g);
        any = true;
    }
    if !any {
        content.append(&gtk::Label::new(Some(tr(
            "These settings are not available on this system.",
        ))));
    }
    super::scroller(&content)
}

/// A row that opens one of the previous Settings' panels, for what has no native version yet.
pub fn legacy_link(title: &str, subtitle: &str, panel: &'static str) -> ndl::ActionRow {
    let row = ndl::ActionRow::builder().title(title).build();
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }
    row.set_activatable(true);
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    row.connect_activated(move |row| crate::legacy::open_panel(row, panel));
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_gets_the_icon_of_what_it_talks_about() {
        assert_eq!(icon_for("tap-to-click"), Some("input-touchpad-symbolic"));
        assert_eq!(
            icon_for("night-light-enabled"),
            Some("night-light-symbolic")
        );
        assert_eq!(
            icon_for("clock-show-seconds"),
            Some("preferences-system-time-symbolic")
        );
        assert_eq!(
            icon_for("lock-enabled"),
            Some("system-lock-screen-symbolic")
        );
        assert_eq!(icon_for("something-unknown"), None);
    }

    #[test]
    fn values_match_variants_of_their_own_type() {
        assert!(Val::S("a").matches(&"a".to_variant()));
        assert!(!Val::S("a").matches(&"b".to_variant()));
        assert!(Val::I(5).matches(&5i32.to_variant()));
        assert!(Val::I(5).matches(&5u32.to_variant()));
        assert!(!Val::I(5).matches(&"5".to_variant()));
    }

    #[test]
    fn a_value_is_written_with_the_type_of_the_key() {
        let like = 1u32.to_variant();
        assert_eq!(Val::I(30).to_variant(&like), Some(30u32.to_variant()));
        // Negative for an unsigned key, or text for a number: nothing is written.
        assert_eq!(Val::I(-1).to_variant(&like), None);
        assert_eq!(Val::S("x").to_variant(&like), None);
        assert_eq!(
            Val::I(-1).to_variant(&0i32.to_variant()),
            Some((-1i32).to_variant())
        );
    }

    #[test]
    fn the_selection_falls_after_the_table_for_an_unlisted_value() {
        let table = [s("A", "a"), s("B", "b")];
        assert_eq!(selected_for(&table, &"b".to_variant()), 1);
        assert_eq!(selected_for(&table, &"zzz".to_variant()), 2);
    }
}
