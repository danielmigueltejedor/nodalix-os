//! Multitasking: hot corner, window tiling and workspaces.

use nodalia_gtk::gtk::{self, prelude::*};

use super::gs;
use crate::i18n::tr;
use crate::rows;

pub fn build() -> gtk::ScrolledWindow {
    let interface = gs::open("org.gnome.desktop.interface");
    let mutter = gs::open("org.gnome.mutter");
    let wm = gs::open("org.gnome.desktop.wm.preferences");
    let switcher = gs::open("org.gnome.shell.app-switcher");

    let overview = interface.as_ref().and_then(|g| {
        gs::group(
            "",
            rows![g.switch(
                "enable-hot-corners",
                tr("Hot corner"),
                tr("Move the pointer to the top-left corner to open the overview"),
            )],
        )
    });
    let windows = mutter.as_ref().and_then(|g| {
        gs::group(
            tr("Windows"),
            rows![g.switch(
                "edge-tiling",
                tr("Tile windows at the screen edges"),
                tr("Drag a window to a side to fill half of the screen"),
            )],
        )
    });
    let workspaces = {
        let dynamic = mutter.as_ref().and_then(|g| {
            g.switch(
                "dynamic-workspaces",
                tr("Dynamic workspaces"),
                tr("Workspaces appear as you use them"),
            )
        });
        let count = wm.as_ref().and_then(|g| {
            g.spin(
                "num-workspaces",
                tr("Number of workspaces"),
                (1.0, 36.0, 1.0),
            )
        });
        if let (Some(dynamic), Some(count)) = (&dynamic, &count) {
            // A fixed number only applies when the workspaces are not dynamic.
            dynamic
                .bind_property("active", count, "sensitive")
                .invert_boolean()
                .sync_create()
                .build();
        }
        let only_primary = mutter.as_ref().and_then(|g| {
            g.switch(
                "workspaces-only-on-primary",
                tr("Workspaces on the main screen only"),
                "",
            )
        });
        let current_only = switcher.as_ref().and_then(|g| {
            g.switch(
                "current-workspace-only",
                tr("Switch apps from the current workspace only"),
                "",
            )
        });
        gs::group(
            tr("Workspaces"),
            rows![dynamic, count, only_primary, current_only],
        )
    };
    gs::page(vec![overview, windows, workspaces])
}
