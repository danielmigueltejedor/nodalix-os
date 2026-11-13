//! Accessibility: seeing, hearing, typing and pointing.

use nodalia_gtk::gtk::{self};

use super::gs::{self, i};
use crate::i18n::tr;
use crate::rows;

const CURSOR_SIZE: [(&str, gs::Val); 5] = [
    i("Default", 24),
    i("Medium", 32),
    i("Large", 48),
    i("Larger", 64),
    i("Largest", 96),
];

pub fn build() -> gtk::ScrolledWindow {
    let a11y_interface = gs::open("org.gnome.desktop.a11y.interface");
    let interface = gs::open("org.gnome.desktop.interface");
    let apps = gs::open("org.gnome.desktop.a11y.applications");
    let wm = gs::open("org.gnome.desktop.wm.preferences");
    let sound = gs::open("org.gnome.desktop.sound");
    let keyboard = gs::open("org.gnome.desktop.a11y.keyboard");
    let mouse = gs::open("org.gnome.desktop.a11y.mouse");

    let seeing = gs::group(
        tr("Seeing"),
        rows![
            a11y_interface.as_ref().and_then(|g| g.switch(
                "high-contrast",
                tr("High contrast"),
                ""
            )),
            a11y_interface.as_ref().and_then(|g| g.switch(
                "show-status-shapes",
                tr("Show shapes on switches"),
                tr("Add an on/off mark to switches")
            )),
            interface.as_ref().and_then(|g| g.choice(
                "cursor-size",
                tr("Cursor size"),
                &CURSOR_SIZE
            )),
            interface.as_ref().and_then(|g| g.switch(
                "cursor-blink",
                tr("Blinking text cursor"),
                ""
            )),
            apps.as_ref()
                .and_then(|g| g.switch("screen-reader-enabled", tr("Screen reader"), "")),
            apps.as_ref()
                .and_then(|g| g.switch("screen-magnifier-enabled", tr("Zoom"), "")),
        ],
    );
    let hearing = gs::group(
        tr("Hearing"),
        rows![
            wm.as_ref().and_then(|g| g.switch(
                "visual-bell",
                tr("Visual alerts"),
                tr("Flash the window or the screen for an alert sound")
            )),
            sound.as_ref().and_then(|g| g.switch(
                "allow-volume-above-100-percent",
                tr("Over-amplification"),
                tr("Allow a volume above 100%")
            )),
        ],
    );
    let typing = gs::group(
        tr("Typing"),
        rows![
            apps.as_ref().and_then(|g| g.switch(
                "screen-keyboard-enabled",
                tr("Screen keyboard"),
                ""
            )),
            keyboard.as_ref().and_then(|g| g.switch(
                "stickykeys-enable",
                tr("Sticky keys"),
                tr("Type shortcuts one key at a time")
            )),
            keyboard.as_ref().and_then(|g| g.switch(
                "slowkeys-enable",
                tr("Slow keys"),
                tr("Ignore keys that are pressed only briefly")
            )),
            keyboard.as_ref().and_then(|g| g.spin(
                "slowkeys-delay",
                tr("Slow keys delay (ms)"),
                (0.0, 3000.0, 50.0)
            )),
            keyboard.as_ref().and_then(|g| g.switch(
                "bouncekeys-enable",
                tr("Bounce keys"),
                tr("Ignore a key pressed again too quickly")
            )),
            keyboard.as_ref().and_then(|g| g.spin(
                "bouncekeys-delay",
                tr("Bounce keys delay (ms)"),
                (0.0, 3000.0, 50.0)
            )),
            keyboard.as_ref().and_then(|g| g.switch(
                "togglekeys-enable",
                tr("Toggle keys"),
                tr("Beep when Caps Lock or Num Lock changes")
            )),
        ],
    );
    let pointing = gs::group(
        tr("Pointing and clicking"),
        rows![
            interface.as_ref().and_then(|g| g.switch(
                "locate-pointer",
                tr("Locate the pointer"),
                tr("Press Ctrl to highlight the pointer")
            )),
            keyboard.as_ref().and_then(|g| g.switch(
                "mousekeys-enable",
                tr("Mouse keys"),
                tr("Move the pointer with the numeric keypad")
            )),
            mouse.as_ref().and_then(|g| g.switch(
                "secondary-click-enabled",
                tr("Simulated secondary click"),
                tr("Hold the main button to right-click")
            )),
            mouse.as_ref().and_then(|g| g.switch(
                "dwell-click-enabled",
                tr("Hover click"),
                tr("Click by holding the pointer still")
            )),
        ],
    );
    gs::page(vec![seeing, hearing, typing, pointing])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_is_translated() {
        for (label, _) in &CURSOR_SIZE {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
