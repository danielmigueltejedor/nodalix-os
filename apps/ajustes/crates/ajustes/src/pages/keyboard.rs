//! Keyboard: key repeat. Input sources and shortcuts are still the previous panel.

use nodalia_gtk::gtk::{self};

use super::gs;
use crate::i18n::tr;
use crate::rows;

pub fn build() -> gtk::ScrolledWindow {
    let keyboard = gs::open("org.gnome.desktop.peripherals.keyboard");
    let typing = keyboard.as_ref().and_then(|g| {
        gs::group(
            tr("Typing"),
            rows![
                g.switch(
                    "repeat",
                    tr("Repeat keys"),
                    tr("Hold a key to type it again")
                ),
                g.spin(
                    "delay",
                    tr("Delay before repeating (ms)"),
                    (100.0, 2000.0, 50.0)
                ),
                g.spin(
                    "repeat-interval",
                    tr("Time between repeats (ms)"),
                    (10.0, 300.0, 5.0)
                ),
                g.switch("remember-numlock-state", tr("Remember Num Lock"), ""),
            ],
        )
    });
    let more = gs::group(
        "",
        rows![Some(gs::legacy_link(
            tr("Input sources and shortcuts"),
            tr("Layouts, compose key and keyboard shortcuts"),
            "keyboard",
        ))],
    );
    gs::page(vec![typing, more])
}
