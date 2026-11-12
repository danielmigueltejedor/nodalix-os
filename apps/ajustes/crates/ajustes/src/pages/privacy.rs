//! Privacy and security: screen lock, file history, clean-up, USB protection and reports.

use nodalia_gtk::gtk::{self};

use super::gs::{self, i, s};
use crate::i18n::tr;
use crate::rows;

pub const BLANK: [(&str, gs::Val); 10] = [
    i("1 minute", 60),
    i("2 minutes", 120),
    i("3 minutes", 180),
    i("4 minutes", 240),
    i("5 minutes", 300),
    i("8 minutes", 480),
    i("10 minutes", 600),
    i("12 minutes", 720),
    i("15 minutes", 900),
    i("Never", 0),
];
const LOCK_DELAY: [(&str, gs::Val); 7] = [
    i("Immediately", 0),
    i("30 seconds", 30),
    i("1 minute", 60),
    i("3 minutes", 180),
    i("5 minutes", 300),
    i("30 minutes", 1800),
    i("1 hour", 3600),
];
const HISTORY: [(&str, gs::Val); 4] = [
    i("1 day", 1),
    i("7 days", 7),
    i("30 days", 30),
    i("Forever", -1),
];
const OLD_FILES: [(&str, gs::Val); 3] = [i("1 day", 1), i("7 days", 7), i("30 days", 30)];
const USB: [(&str, gs::Val); 2] = [
    s("When the screen is locked", "lockscreen"),
    s("Always", "always"),
];

pub fn build() -> gtk::ScrolledWindow {
    let screensaver = gs::open("org.gnome.desktop.screensaver");
    let session = gs::open("org.gnome.desktop.session");
    let privacy = gs::open("org.gnome.desktop.privacy");
    let location = gs::open("org.gnome.system.location");

    let lock = {
        let blank = session
            .as_ref()
            .and_then(|g| g.choice("idle-delay", tr("Turn the screen off after"), &BLANK));
        let enabled = screensaver
            .as_ref()
            .and_then(|g| g.switch("lock-enabled", tr("Lock the screen automatically"), ""));
        let delay = screensaver.as_ref().and_then(|g| {
            g.choice(
                "lock-delay",
                tr("Lock after the screen turns off"),
                &LOCK_DELAY,
            )
        });
        gs::group(tr("Screen lock"), rows![blank, enabled, delay])
    };
    let files = privacy.as_ref().and_then(|g| {
        gs::group(
            tr("File history and trash"),
            rows![
                g.switch(
                    "remember-recent-files",
                    tr("File history"),
                    tr("Remember the files you open")
                ),
                g.choice("recent-files-max-age", tr("Keep the history for"), &HISTORY),
                g.switch(
                    "remove-old-trash-files",
                    tr("Empty the trash automatically"),
                    ""
                ),
                g.switch(
                    "remove-old-temp-files",
                    tr("Delete temporary files automatically"),
                    ""
                ),
                g.choice("old-files-age", tr("Delete files older than"), &OLD_FILES),
            ],
        )
    });
    let device = {
        let usb = privacy.as_ref().and_then(|g| {
            g.switch(
                "usb-protection",
                tr("USB protection"),
                tr("Block new USB devices while the screen is locked"),
            )
        });
        let level = privacy
            .as_ref()
            .and_then(|g| g.choice("usb-protection-level", tr("Block USB devices"), &USB));
        let location = location.as_ref().and_then(|g| {
            g.switch(
                "enabled",
                tr("Location services"),
                tr("Let apps know where you are"),
            )
        });
        gs::group(tr("Devices"), rows![usb, level, location])
    };
    let reports = privacy.as_ref().and_then(|g| {
        gs::group(
            tr("Reports"),
            rows![
                g.switch("report-technical-problems", tr("Send problem reports"), ""),
                g.switch(
                    "remember-app-usage",
                    tr("Remember app usage"),
                    tr("Used to sort the app list")
                ),
            ],
        )
    });
    gs::page(vec![lock, files, device, reports])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_is_translated() {
        for (label, _) in BLANK
            .iter()
            .chain(&LOCK_DELAY)
            .chain(&HISTORY)
            .chain(&OLD_FILES)
            .chain(&USB)
        {
            assert!(crate::i18n::has_translation(label), "{label}");
        }
    }
}
