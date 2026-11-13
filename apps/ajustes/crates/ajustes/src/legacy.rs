//! Pages that are still the GNOME Control Center panel with the same name.

use std::process::Command;

use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

use crate::i18n::tr;

/// Opens a panel of the previous Settings. The panel name comes from `ajustes_core::pages`, a
/// fixed list of plain names, and is passed as one argument, never through a shell.
pub fn open_panel(from: &impl IsA<gtk::Widget>, panel: &str) {
    match Command::new("gnome-control-center").arg(panel).spawn() {
        // A child that is never waited for stays a zombie after it exits. A short-lived thread
        // waits for it, so the GTK thread never does.
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(error) => report_launch_error(from, panel, &error),
    }
}

fn report_launch_error(from: &impl IsA<gtk::Widget>, panel: &str, error: &std::io::Error) {
    eprintln!("gnome-control-center {panel}: {error}");
    if let Some(overlay) = from
        .ancestor(ndl::ToastOverlay::static_type())
        .and_downcast::<ndl::ToastOverlay>()
    {
        overlay.add_toast(ndl::Toast::new(&format!(
            "{}: {error}",
            tr("Could not open the previous Settings")
        )));
    }
}

/// What the content area shows for such a page.
pub fn page(title: &str, icon: &str, panel: &'static str) -> ndl::StatusPage {
    let status = ndl::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(tr("This page has not been ported to the new Settings yet. It opens in the previous Settings window."))
        .build();
    let open = gtk::Button::with_label(tr("Open"));
    open.add_css_class("suggested-action");
    open.add_css_class("pill");
    open.set_halign(gtk::Align::Center);
    open.connect_clicked(move |b| open_panel(b, panel));
    status.set_child(Some(&open));
    status
}
