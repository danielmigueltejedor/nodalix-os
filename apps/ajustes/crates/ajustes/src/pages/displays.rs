//! Displays: which screens are connected and how they are set, read from mutter, plus Night Light.
//! Changing resolution, scale or arrangement stays in the previous panel: a wrong configuration
//! can leave a screen unusable, and the previous panel asks to confirm it.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{glib, prelude::*},
};

use super::gs;
use super::{bus, column, scroller};
use crate::i18n::tr;
use crate::rows;

const MUTTER: &str = "org.gnome.Mutter.DisplayConfig";
const MUTTER_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const COLOR: &str = "org.gnome.settings-daemon.plugins.color";

/// A value of an `a{sv}` dictionary.
fn lookup(dict: &glib::Variant, key: &str) -> Option<glib::Variant> {
    glib::VariantDict::new(Some(dict)).lookup_value(key, None)
}

/// A connected screen as the page shows it.
#[derive(Debug, PartialEq)]
pub struct Screen {
    pub connector: String,
    pub name: String,
    pub mode: String,
    pub scale: f64,
}

/// `1920×1080 · 60 Hz` for a mode.
fn mode_text(width: i32, height: i32, refresh: f64) -> String {
    format!("{width}×{height} · {} Hz", refresh.round() as i64)
}

/// The screens in mutter's `GetCurrentState` reply `(u, a((ssss)a(siiddada{sv})a{sv}), a(...), a{sv})`.
/// What does not parse is left out, so a changed API shows fewer screens, not a crash.
pub fn parse_state(state: &glib::Variant) -> Vec<Screen> {
    let mut out = Vec::new();
    let Some(monitors) = state.try_child_value(1) else {
        return out;
    };
    for monitor in monitors.iter() {
        let Some(ids) = monitor.try_child_value(0) else {
            continue;
        };
        let text = |i: usize| {
            ids.try_child_value(i)
                .and_then(|v| v.str().map(str::to_owned))
                .unwrap_or_default()
        };
        let (connector, vendor, product) = (text(0), text(1), text(2));
        let props = monitor.try_child_value(2);
        let display_name = props
            .as_ref()
            .and_then(|p| lookup(p, "display-name"))
            .and_then(|v| v.str().map(str::to_owned));
        let name = display_name.unwrap_or_else(|| format!("{vendor} {product}").trim().to_owned());
        let mut mode = String::new();
        let mut scale = 1.0;
        if let Some(modes) = monitor.try_child_value(1) {
            for m in modes.iter() {
                let current = m
                    .try_child_value(6)
                    .and_then(|p| lookup(&p, "is-current"))
                    .and_then(|v| v.get::<bool>())
                    .unwrap_or(false);
                if !current {
                    continue;
                }
                let w = m.try_child_value(1).and_then(|v| v.get::<i32>());
                let h = m.try_child_value(2).and_then(|v| v.get::<i32>());
                let r = m.try_child_value(3).and_then(|v| v.get::<f64>());
                if let (Some(w), Some(h), Some(r)) = (w, h, r) {
                    mode = mode_text(w, h, r);
                }
            }
        }
        // The scale is a property of the logical monitor that shows this connector.
        if let Some(logical) = state.try_child_value(2) {
            for l in logical.iter() {
                let uses = l.try_child_value(5).is_some_and(|ms| {
                    ms.iter().any(|m| {
                        m.try_child_value(0)
                            .and_then(|v| v.str().map(|s| s == connector))
                            .unwrap_or(false)
                    })
                });
                if uses {
                    scale = l
                        .try_child_value(2)
                        .and_then(|v| v.get::<f64>())
                        .unwrap_or(1.0);
                }
            }
        }
        out.push(Screen {
            connector,
            name,
            mode,
            scale,
        });
    }
    out
}

fn screen_row(screen: &Screen) -> ndl::ActionRow {
    let row = ndl::ActionRow::builder()
        .title(if screen.name.is_empty() {
            &screen.connector
        } else {
            &screen.name
        })
        .build();
    let scale = (screen.scale * 100.0).round() as i64;
    let mut detail = screen.mode.clone();
    if !detail.is_empty() {
        detail.push_str(&format!(" · {scale} %"));
    }
    row.set_subtitle(format!("{}  {detail}", screen.connector).trim());
    row.add_prefix(&gtk::Image::from_icon_name("video-display-symbolic"));
    row
}

pub fn build() -> gtk::ScrolledWindow {
    let content = column();
    let screens = ndl::PreferencesGroup::builder()
        .title(tr("Screens"))
        .build();
    let placeholder = ndl::ActionRow::builder()
        .title(tr("Looking for screens…"))
        .build();
    screens.add(&placeholder);
    content.append(&screens);

    let night = gs::open(COLOR);
    let night_group = night.as_ref().and_then(|g| {
        gs::group(
            tr("Night Light"),
            rows![
                g.switch(
                    "night-light-enabled",
                    tr("Turn on"),
                    tr("Warmer colors in the evening")
                ),
                g.switch(
                    "night-light-schedule-automatic",
                    tr("Sunset to sunrise"),
                    ""
                ),
                g.spin(
                    "night-light-schedule-from",
                    tr("From (hour)"),
                    (0.0, 23.5, 0.5)
                ),
                g.spin("night-light-schedule-to", tr("To (hour)"), (0.0, 23.5, 0.5)),
                g.slider(
                    "night-light-temperature",
                    tr("Color temperature"),
                    tr("Warmer to the right"),
                    (1700.0, 4700.0, 100.0)
                ),
            ],
        )
    });
    if let Some(group) = &night_group {
        content.append(group);
    }
    if let Some(more) = gs::group(
        "",
        rows![Some(gs::legacy_link(
            tr("Resolution, scale and arrangement"),
            tr("Change how each screen is set up"),
            "display",
        ))],
    ) {
        content.append(&more);
    }

    glib::spawn_future_local(async move {
        let found = async {
            let connection = bus::session().await?;
            let reply = bus::call(
                &connection,
                MUTTER,
                MUTTER_PATH,
                MUTTER,
                "GetCurrentState",
                None,
            )
            .await
            .ok()?;
            Some(parse_state(&reply))
        }
        .await;
        screens.remove(&placeholder);
        match found {
            Some(list) if !list.is_empty() => {
                for screen in &list {
                    screens.add(&screen_row(screen));
                }
            }
            _ => {
                let row = ndl::ActionRow::builder()
                    .title(tr("The screens could not be read"))
                    .build();
                screens.add(&row);
            }
        }
    });
    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> glib::Variant {
        // One screen at 1920×1080, 59.94 Hz, scaled 1.25, plus a mode that is not current.
        let mode = |w: i32, h: i32, r: f64, current: bool| {
            let mut props = std::collections::HashMap::new();
            if current {
                props.insert("is-current".to_owned(), true.to_variant());
            }
            ("mode".to_owned(), w, h, r, 1.0f64, vec![1.0f64], props)
        };
        let mut monitor_props = std::collections::HashMap::new();
        monitor_props.insert("display-name".to_owned(), "Dell U2720Q".to_variant());
        let monitors = vec![(
            (
                "DP-1".to_owned(),
                "DEL".to_owned(),
                "U2720Q".to_owned(),
                "1".to_owned(),
            ),
            vec![mode(3840, 2160, 60.0, false), mode(1920, 1080, 59.94, true)],
            monitor_props,
        )];
        let logical = vec![(
            0i32,
            0i32,
            1.25f64,
            0u32,
            true,
            vec![(
                "DP-1".to_owned(),
                "DEL".to_owned(),
                "U2720Q".to_owned(),
                "1".to_owned(),
            )],
            std::collections::HashMap::<String, glib::Variant>::new(),
        )];
        (
            1u32,
            monitors,
            logical,
            std::collections::HashMap::<String, glib::Variant>::new(),
        )
            .to_variant()
    }

    #[test]
    fn the_current_mode_and_scale_of_a_screen_are_read() {
        let screens = parse_state(&state());
        assert_eq!(
            screens,
            vec![Screen {
                connector: "DP-1".into(),
                name: "Dell U2720Q".into(),
                mode: "1920×1080 · 60 Hz".into(),
                scale: 1.25,
            }]
        );
    }

    #[test]
    fn a_reply_of_another_shape_gives_no_screens() {
        assert!(parse_state(&"nothing".to_variant()).is_empty());
    }
}
