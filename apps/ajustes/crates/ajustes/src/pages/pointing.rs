//! Mouse and touchpad.

use nodalia_gtk::gtk;

use super::gs::{self, s};
use crate::i18n::tr;
use crate::rows;

const ACCELERATION: [(&str, gs::Val); 3] = [
    s("Default", "default"),
    s("Flat", "flat"),
    s("Adaptive", "adaptive"),
];
const TOUCHPAD_EVENTS: [(&str, gs::Val); 3] = [
    s("On", "enabled"),
    s("Off", "disabled"),
    s(
        "Off while a mouse is connected",
        "disabled-on-external-mouse",
    ),
];
const CLICK_METHOD: [(&str, gs::Val); 3] = [
    s("Default", "default"),
    s("Press with two fingers to right-click", "fingers"),
    s("Press in the bottom corners to right-click", "areas"),
];

pub fn build() -> gtk::ScrolledWindow {
    let mouse = gs::open("org.gnome.desktop.peripherals.mouse");
    let touchpad = gs::open("org.gnome.desktop.peripherals.touchpad");

    let mouse_group = mouse.as_ref().and_then(|g| {
        gs::group(
            tr("Mouse"),
            rows![
                g.switch(
                    "left-handed",
                    tr("Left-handed"),
                    tr("Swap the main and secondary buttons")
                ),
                g.switch(
                    "natural-scroll",
                    tr("Natural scrolling"),
                    tr("Content follows the scroll")
                ),
                g.slider("speed", tr("Pointer speed"), "", (-1.0, 1.0, 0.1)),
                g.choice("accel-profile", tr("Acceleration"), &ACCELERATION),
                g.spin(
                    "double-click",
                    tr("Double-click time (ms)"),
                    (100.0, 1000.0, 50.0)
                ),
            ],
        )
    });
    let touchpad_group = touchpad.as_ref().and_then(|g| {
        gs::group(
            tr("Touchpad"),
            rows![
                g.choice("send-events", tr("Touchpad"), &TOUCHPAD_EVENTS),
                g.switch("tap-to-click", tr("Tap to click"), ""),
                g.switch(
                    "two-finger-scrolling-enabled",
                    tr("Two-finger scrolling"),
                    ""
                ),
                g.switch("edge-scrolling-enabled", tr("Edge scrolling"), ""),
                g.switch(
                    "natural-scroll",
                    tr("Natural scrolling"),
                    tr("Content follows the scroll")
                ),
                g.switch("disable-while-typing", tr("Disable while typing"), ""),
                g.slider("speed", tr("Pointer speed"), "", (-1.0, 1.0, 0.1)),
                g.choice("click-method", tr("Secondary click"), &CLICK_METHOD),
            ],
        )
    });
    gs::page(vec![mouse_group, touchpad_group])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_is_translated() {
        for (label, _) in ACCELERATION
            .iter()
            .chain(&TOUCHPAD_EVENTS)
            .chain(&CLICK_METHOD)
        {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
