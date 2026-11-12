//! Printers: the ones CUPS knows, whether each is ready, the default, and a test page. Adding and
//! removing printers stays in the previous panel.

use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::printers::{self, Printer, State};

use super::gs;
use super::{column, scroller};
use crate::i18n::tr;
use crate::rows;

type Reload = Rc<dyn Fn()>;

/// Runs a command (never through a shell) and returns what it printed, or `None` if it could not
/// run or failed.
async fn run(argv: &[&str]) -> Option<String> {
    let process = gio::Subprocess::newv(
        &argv.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
    )
    .ok()?;
    let (out, _) = process.communicate_utf8_future(None).await.ok()?;
    process
        .is_successful()
        .then(|| out.map(|s| s.to_string()))?
}

fn state_text(state: State) -> &'static str {
    match state {
        State::Idle => "Ready",
        State::Printing => "Printing",
        State::Disabled => "Not available",
    }
}

fn printer_row(
    printer: &Printer,
    toasts: &ndl::ToastOverlay,
    reload: &Reload,
) -> ndl::ActionRow {
    let row = ndl::ActionRow::builder().title(printer.title()).build();
    let mut subtitle = tr(state_text(printer.state)).to_owned();
    if printer.is_default {
        subtitle.push_str(" · ");
        subtitle.push_str(tr("Default"));
    }
    row.set_subtitle(&subtitle);
    row.add_prefix(&gtk::Image::from_icon_name("printer-symbolic"));
    if !printers::is_valid_name(&printer.name) {
        return row;
    }
    if !printer.is_default {
        let button = gtk::Button::with_label(tr("Make default"));
        button.set_valign(gtk::Align::Center);
        let (name, reload, toasts) = (printer.name.clone(), reload.clone(), toasts.clone());
        button.connect_clicked(move |_| {
            let (name, reload, toasts) = (name.clone(), reload.clone(), toasts.clone());
            glib::spawn_future_local(async move {
                if run(&["lpoptions", "-d", &name]).await.is_none() {
                    toasts.add_toast(ndl::Toast::new(tr("Could not change the default printer")));
                }
                reload();
            });
        });
        row.add_suffix(&button);
    }
    if printer.state != State::Disabled {
        let button = gtk::Button::with_label(tr("Test page"));
        button.set_valign(gtk::Align::Center);
        let (name, toasts) = (printer.name.clone(), toasts.clone());
        button.connect_clicked(move |_| {
            let (name, toasts) = (name.clone(), toasts.clone());
            glib::spawn_future_local(async move {
                let ok = run(&["lp", "-d", &name, "/usr/share/cups/data/testprint"])
                    .await
                    .is_some();
                toasts.add_toast(ndl::Toast::new(tr(if ok {
                    "Test page sent"
                } else {
                    "The test page could not be sent"
                })));
            });
        });
        row.add_suffix(&button);
    }
    row
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Printers"))
        .build();
    content.append(&group);
    if let Some(more) = gs::group(
        "",
        rows![Some(gs::legacy_link(
            tr("Add or remove printers"),
            tr("Find printers on the network and set them up"),
            "printers",
        ))],
    ) {
        content.append(&more);
    }

    let rows_now: Rc<std::cell::RefCell<Vec<gtk::Widget>>> = Rc::default();
    let reload: Rc<std::cell::RefCell<Option<Reload>>> = Rc::default();
    let toasts = toasts.clone();
    let load: Reload = {
        let (group, rows_now, reload) = (group.clone(), rows_now.clone(), reload.clone());
        Rc::new(move || {
            let (group, rows_now, reload, toasts) = (
                group.clone(),
                rows_now.clone(),
                reload.clone(),
                toasts.clone(),
            );
            glib::spawn_future_local(async move {
                let list = run(&["lpstat", "-p"]).await;
                let default = run(&["lpstat", "-d"]).await.unwrap_or_default();
                for old in rows_now.borrow_mut().drain(..) {
                    group.remove(&old);
                }
                let Some(list) = list else {
                    // `lpstat` fails when CUPS is not running, and also when there is no printer.
                    let row = ndl::ActionRow::builder()
                        .title(tr("No printers set up"))
                        .subtitle(tr("Add one with the button below"))
                        .build();
                    group.add(&row);
                    rows_now.borrow_mut().push(row.upcast());
                    return;
                };
                let found = printers::parse(&list, &default);
                let Some(reload) = reload.borrow().clone() else {
                    return;
                };
                if found.is_empty() {
                    let row = ndl::ActionRow::builder()
                        .title(tr("No printers set up"))
                        .subtitle(tr("Add one with the button below"))
                        .build();
                    group.add(&row);
                    rows_now.borrow_mut().push(row.upcast());
                }
                for printer in &found {
                    let row = printer_row(printer, &toasts, &reload);
                    group.add(&row);
                    rows_now.borrow_mut().push(row.upcast());
                }
            });
        })
    };
    *reload.borrow_mut() = Some(load.clone());
    load();
    // The state of a printer changes while the page is open (a job finishes): look again.
    let weak = content.downgrade();
    glib::timeout_add_seconds_local(8, move || {
        let Some(content) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if content.is_mapped() {
            load();
        }
        glib::ControlFlow::Continue
    });
    scroller(&content)
}
