//! More icons in NDL's drawing, for Settings and the system monitor: the networks, the batteries,
//! the devices that connect to a computer and a few of the places. Drawn like the rest (see
//! `icons`): a 24 px grid, a 2.25 px round stroke, the foreground colour.

use crate::icons::Part::{Circle, Faint, Line, Rect, Solid};
use crate::icons::{svg, FOLDER};

/// The three arcs of the Wi-Fi mark, the widest first, and its dot.
const ARC_WIDE: &str = "M2 8.82a15 15 0 0 1 20 0";
const ARC_MID: &str = "M5 12.86a10 10 0 0 1 14 0";
const ARC_NEAR: &str = "M8.5 16.43a5 5 0 0 1 7 0";
const DOT: &str = "M12 20h.01";
const SLASH: &str = "M3 3l18 18";

/// A padlock, shared by the names that mean one.
const PADLOCK_BODY: (f32, f32, f32, f32, f32) = (4.5, 10.5, 15.0, 10.0, 2.0);
const PADLOCK_SHACKLE: &str = "M8 10.5V7a4 4 0 0 1 8 0v3.5";

/// The charge of a battery as the bar inside it: one path for each tenth.
const CHARGE: [&str; 10] = [
    "M6 11h1v2H6z",
    "M6 11h2v2H6z",
    "M6 11h3v2H6z",
    "M6 11h4v2H6z",
    "M6 11h5v2H6z",
    "M6 11h6v2H6z",
    "M6 11h7v2H6z",
    "M6 11h8v2H6z",
    "M6 11h9v2H6z",
    "M6 11h10v2H6z",
];

const BATTERY_NAMES: [&str; 11] = [
    "battery-level-0-symbolic",
    "battery-level-10-symbolic",
    "battery-level-20-symbolic",
    "battery-level-30-symbolic",
    "battery-level-40-symbolic",
    "battery-level-50-symbolic",
    "battery-level-60-symbolic",
    "battery-level-70-symbolic",
    "battery-level-80-symbolic",
    "battery-level-90-symbolic",
    "battery-level-100-symbolic",
];

fn padlock() -> String {
    let (x, y, w, h, r) = PADLOCK_BODY;
    svg(&[Rect(x, y, w, h, r), Line(PADLOCK_SHACKLE)])
}

fn battery(tenths: usize) -> String {
    let outline = [Rect(2.5, 7.0, 16.0, 10.0, 2.0), Line("M21.5 11v2")];
    match tenths {
        0 => svg(&outline),
        n => {
            let mut parts = outline.into_iter().collect::<Vec<_>>();
            parts.push(Solid(CHARGE[n - 1]));
            svg(&parts)
        }
    }
}

pub(crate) fn drawings() -> Vec<(&'static str, String)> {
    let mut all: Vec<(&'static str, String)> = BATTERY_NAMES
        .iter()
        .enumerate()
        .map(|(tenths, name)| (*name, battery(tenths)))
        .collect();
    all.extend([
        // Networks.
        ("network-wireless-signal-excellent-symbolic", svg(&[Line(ARC_WIDE), Line(ARC_MID), Line(ARC_NEAR), Line(DOT)])),
        ("network-wireless-connected-symbolic", svg(&[Line(ARC_WIDE), Line(ARC_MID), Line(ARC_NEAR), Line(DOT)])),
        ("network-wireless-signal-good-symbolic", svg(&[Faint(ARC_WIDE), Line(ARC_MID), Line(ARC_NEAR), Line(DOT)])),
        ("network-wireless-signal-ok-symbolic", svg(&[Faint(ARC_WIDE), Faint(ARC_MID), Line(ARC_NEAR), Line(DOT)])),
        ("network-wireless-signal-weak-symbolic", svg(&[Faint(ARC_WIDE), Faint(ARC_MID), Faint(ARC_NEAR), Line(DOT)])),
        ("network-wireless-signal-none-symbolic", svg(&[Faint(ARC_WIDE), Faint(ARC_MID), Faint(ARC_NEAR), Faint(DOT)])),
        ("network-wireless-disabled-symbolic", svg(&[Faint(ARC_WIDE), Faint(ARC_MID), Faint(ARC_NEAR), Faint(DOT), Line(SLASH)])),
        ("network-wireless-offline-symbolic", svg(&[Faint(ARC_WIDE), Faint(ARC_MID), Faint(ARC_NEAR), Faint(DOT), Line(SLASH)])),
        ("network-wireless-acquiring-symbolic", svg(&[Line(ARC_WIDE), Line(ARC_MID), Line(ARC_NEAR)])),
        ("network-wired-symbolic", svg(&[Rect(16.0, 16.0, 6.0, 6.0, 1.5), Rect(2.0, 16.0, 6.0, 6.0, 1.5), Rect(9.0, 2.0, 6.0, 6.0, 1.5), Line("M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3M12 12V8")])),
        ("network-wired-disconnected-symbolic", svg(&[Rect(16.0, 16.0, 6.0, 6.0, 1.5), Rect(2.0, 16.0, 6.0, 6.0, 1.5), Rect(9.0, 2.0, 6.0, 6.0, 1.5), Line("M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3M12 12V8"), Line(SLASH)])),
        ("network-vpn-symbolic", svg(&[Line("M2 18v3a1 1 0 0 0 1 1h4v-3h3v-3h2l1.4-1.4a6.5 6.5 0 1 0-4-4z"), Line("M16.5 7.5h.01")])),
        ("network-error-symbolic", svg(&[Circle(12.0, 12.0, 9.0), Line("M12 7.5v5M12 16.5h.01")])),
        ("folder-remote-symbolic", svg(&[Line(FOLDER), Line("M9 15h6M12.5 12.5 15 15l-2.5 2.5")])),
        ("weather-overcast-symbolic", svg(&[Line("M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9z")])),
        ("weather-clear-night-symbolic", svg(&[Line("M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z")])),
        ("night-light-symbolic", svg(&[Line("M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z")])),
        // Devices.
        ("ac-adapter-symbolic", svg(&[Line("M12 22v-5M9 8V2M15 8V2M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8z")])),
        ("audio-speakers-symbolic", svg(&[Rect(5.0, 2.5, 14.0, 19.0, 2.5), Circle(12.0, 15.0, 3.5), Line("M12 6.5h.01")])),
        ("audio-card-symbolic", svg(&[Rect(3.0, 5.0, 18.0, 14.0, 2.5), Circle(12.0, 12.0, 3.5), Line("M6.5 8.5h.01")])),
        ("audio-headset-symbolic", svg(&[Line("M4 15v-3a8 8 0 0 1 16 0v3"), Rect(3.0, 14.0, 4.0, 6.0, 1.5), Rect(17.0, 14.0, 4.0, 6.0, 1.5)])),
        ("camera-photo-symbolic", svg(&[Line("M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3z"), Circle(12.0, 13.0, 3.5)])),
        ("camera-video-symbolic", svg(&[Rect(2.0, 6.0, 14.0, 12.0, 2.0), Line("m16 13 5.2 3.5a.5.5 0 0 0 .8-.4V7.9a.5.5 0 0 0-.8-.4L16 11")])),
        ("display-brightness-symbolic", svg(&[Circle(12.0, 12.0, 4.0), Line("M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4")])),
        ("input-gaming-symbolic", svg(&[Line("M6 11h4M8 9v4M15 12h.01M18 10h.01M17.3 5H6.7a4 4 0 0 0-4 3.6C2.6 9.4 2 14.5 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.4-1.4a2 2 0 0 1 1.4-.6h4.3a2 2 0 0 1 1.4.6L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.5-.6-6.6-.7-7.3A4 4 0 0 0 17.3 5z")])),
        ("input-tablet-symbolic", svg(&[Rect(4.0, 2.5, 16.0, 19.0, 2.5), Line("M11 18.5h2")])),
        ("input-touchpad-symbolic", svg(&[Rect(3.0, 5.0, 18.0, 14.0, 2.5), Line("M3 14h18M12 14v5")])),
        ("media-removable-symbolic", svg(&[Line("M7 2.5h7l4 4v13a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-15a2 2 0 0 1 2-2zM9 6.5v2.5M12 6.5v2.5")])),
        ("microphone-disabled-symbolic", svg(&[Line("M12 19v3M15 9.3V5a3 3 0 0 0-5.7-1.3M9 9v3a3 3 0 0 0 5.1 2.1M19 10v2a7 7 0 0 1-.1 1.2M5 10v2a7 7 0 0 0 12 5"), Line("M2 2l20 20")])),
        ("modem-symbolic", svg(&[Rect(2.0, 14.0, 20.0, 8.0, 2.0), Line("M6 18h.01M10 18h.01M15 10v4M17.8 7.2a4 4 0 0 0-5.7 0M20.7 4.3a8 8 0 0 0-11.4 0")])),
        ("multimedia-player-symbolic", svg(&[Rect(6.0, 2.5, 12.0, 19.0, 2.5), Circle(12.0, 14.0, 3.5), Line("M9.5 6.5h5")])),
        ("phone-symbolic", svg(&[Rect(7.0, 2.0, 10.0, 20.0, 2.5), Line("M11.5 18.5h1")])),
        ("scanner-symbolic", svg(&[Line("M6 11V5h12v6"), Rect(2.5, 11.0, 19.0, 8.0, 2.0), Line("M7 15h.01M17 15h.01")])),
        // Security and sharing.
        ("channel-secure-symbolic", padlock()),
        ("system-lock-screen-symbolic", padlock()),
        ("security-high-symbolic", svg(&[Line("M20 13c0 5-3.5 7.5-7.7 9a1 1 0 0 1-.7 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.2-2.7a1.2 1.2 0 0 1 1.5 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z"), Line("m9 12 2 2 4-4")])),
        ("emblem-shared-symbolic", svg(&[Circle(6.0, 12.0, 2.5), Circle(18.0, 6.0, 2.5), Circle(18.0, 18.0, 2.5), Line("M8.2 10.8l7.6-3.6M8.2 13.2l7.6 3.6")])),
        ("notifications-disabled-symbolic", svg(&[Line("M8.7 3A6 6 0 0 1 18 8a21 21 0 0 0 .6 5M17 17H3s3-2 3-9a4.7 4.7 0 0 1 .3-1.7M10.3 21a1.9 1.9 0 0 0 3.4 0"), Line("M2 2l20 20")])),
        // Power.
        ("power-profile-balanced-symbolic", svg(&[Circle(12.0, 12.0, 9.0), Solid("M12 3a9 9 0 0 1 0 18z")])),
        ("power-profile-power-saver-symbolic", svg(&[Line("M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.5 19 2c1 2 2 4.2 2 8 0 5.5-4.8 10-10 10zM2 21c0-3 1.9-5.4 5.1-6C9.5 14.5 12 13 13 12")])),
        ("power-profile-performance-symbolic", svg(&[Line("M13 2 3 14h9l-1 8 10-12h-9z")])),
        // Files and things the system shows.
        ("image-x-generic-symbolic", svg(&[Rect(3.5, 3.5, 17.0, 17.0, 2.5), Circle(9.0, 9.0, 1.5), Line("m20.5 15.5-4-4L6 20.5")])),
        ("video-x-generic-symbolic", svg(&[Rect(3.0, 4.0, 18.0, 16.0, 2.0), Line("M7 4v16M17 4v16M3 12h18M3 8h4M3 16h4M17 8h4M17 16h4")])),
        ("x-office-document-symbolic", svg(&[Line("M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4")])),
        ("font-x-generic-symbolic", svg(&[Line("M4 20 12 4l8 16M7.5 14h9")])),
        ("format-text-larger-symbolic", svg(&[Line("M3 19 8 6l5 13M4.8 15h6.4M19 7v6M16 10h6")])),
        ("application-x-executable-symbolic", svg(&[Rect(3.5, 4.0, 17.0, 16.0, 2.5), Line("m7.5 9 3 3-3 3M13 15h4")])),
        ("applications-science-symbolic", svg(&[Line("M10 2v7.3L4.5 19a2 2 0 0 0 1.7 3h11.6a2 2 0 0 0 1.7-3L14 9.3V2M8.5 2h7M7 16h10")])),
        ("system-software-install-symbolic", svg(&[Line("M11 21.7a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.7l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.7zM12 22V12M3.3 7 12 12l8.7-5M7.5 4.3l9 5.2")])),
        ("alarm-symbolic", svg(&[Circle(12.0, 13.0, 8.0), Line("M12 9v4l2 2M5 3 2 6M22 6l-3-3")])),
        ("object-rotate-right-symbolic", svg(&[Line("M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5")])),
        ("preferences-desktop-wallpaper-symbolic", svg(&[Rect(2.5, 3.5, 19.0, 13.0, 2.0), Line("M8 21h8M12 16.5V21M2.5 13l4.5-4.5L11.5 13l3-3 7 7")])),
        ("view-dual-symbolic", svg(&[Rect(3.5, 4.5, 17.0, 15.0, 2.0), Line("M12 4.5v15")])),
        ("window-new-symbolic", svg(&[Rect(3.5, 4.5, 17.0, 15.0, 2.5), Line("M3.5 9.5h17M12 12.5v4M10 14.5h4")])),
    ]);
    all
}
