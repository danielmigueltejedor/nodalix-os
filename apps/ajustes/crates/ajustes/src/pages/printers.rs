//! Printers: the ones CUPS knows, whether each is ready, the default, a test page, finding
//! printers on the network, adding one by address and removing one. Setting up goes through
//! `lpadmin` with a driverless (IPP Everywhere) queue; what needs a driver is told, not hidden.

use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::printers::{self, Printer, State};

use super::{column, scroller};
use crate::i18n::tr;

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

/// What a command printed to stderr and whether it worked.
async fn run_full(argv: &[&str]) -> (bool, String) {
    let Ok(process) = gio::Subprocess::newv(
        &argv.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        gio::SubprocessFlags::STDOUT_SILENCE | gio::SubprocessFlags::STDERR_PIPE,
    ) else {
        return (false, String::new());
    };
    let Ok((_, err)) = process.communicate_utf8_future(None).await else {
        return (false, String::new());
    };
    (
        process.is_successful(),
        err.map(|s| s.to_string()).unwrap_or_default(),
    )
}

/// Changes the print system's setup: as the person first, and through `pkexec` (which asks for
/// the password) if CUPS says they may not. The error text is what the tool said.
async fn run_admin(argv: &[&str]) -> Result<(), String> {
    let (ok, err) = run_full(argv).await;
    if ok {
        return Ok(());
    }
    let lower = err.to_lowercase();
    let denied = [
        "forbidden",
        "unauthorized",
        "not authorized",
        "permission denied",
    ]
    .iter()
    .any(|w| lower.contains(w));
    if !denied {
        return Err(err);
    }
    // The elevated tool is looked up in the system folder, never along the user's PATH.
    let tool = format!("/usr/bin/{}", argv.first().copied().unwrap_or_default());
    let mut elevated = vec!["pkexec", tool.as_str()];
    elevated.extend_from_slice(argv.get(1..).unwrap_or_default());
    let (ok, err) = run_full(&elevated).await;
    if ok {
        Ok(())
    } else {
        Err(err)
    }
}

fn state_text(state: State) -> &'static str {
    match state {
        State::Idle => "Ready",
        State::Printing => "Printing",
        State::Disabled => "Not available",
    }
}

fn printer_row(printer: &Printer, toasts: &ndl::ToastOverlay, reload: &Reload) -> ndl::ActionRow {
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
    {
        let button = gtk::Button::from_icon_name("user-trash-symbolic");
        button.set_valign(gtk::Align::Center);
        button.set_tooltip_text(Some(tr("Remove the printer")));
        button.add_css_class("flat");
        let (name, title, reload, toasts) = (
            printer.name.clone(),
            printer.title(),
            reload.clone(),
            toasts.clone(),
        );
        button.connect_clicked(move |button| {
            let (name, title, reload, toasts) =
                (name.clone(), title.clone(), reload.clone(), toasts.clone());
            let window = button.root().and_downcast::<gtk::Window>();
            glib::spawn_future_local(async move {
                let dialog = gtk::AlertDialog::builder()
                    .message(tr("Remove this printer?"))
                    .detail(title)
                    .buttons([tr("Cancel"), tr("Remove")])
                    .cancel_button(0)
                    .default_button(0)
                    .modal(true)
                    .build();
                if dialog.choose_future(window.as_ref()).await != Ok(1) {
                    return;
                }
                if run_admin(&["lpadmin", "-x", &name]).await.is_err() {
                    toasts.add_toast(ndl::Toast::new(tr("The printer could not be removed")));
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

/// Sets up a printer at `uri` as a driverless queue and says how it went.
async fn add_printer(
    title: &str,
    uri: &str,
    names: &Rc<std::cell::RefCell<Vec<String>>>,
    toasts: &ndl::ToastOverlay,
    reload: &Reload,
) {
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    let name = printers::queue_name(&title, &names.borrow());
    let result = run_admin(&[
        "lpadmin",
        "-p",
        &name,
        "-E",
        "-v",
        uri,
        "-m",
        "everywhere",
        "-D",
        &title,
    ])
    .await;
    toasts.add_toast(ndl::Toast::new(tr(match result {
        Ok(()) => "Printer added",
        Err(_) => "The printer could not be set up; it may need a driver",
    })));
    reload();
}

/// Search for printers on the network, and add one by its address.
fn add_group(
    toasts: &ndl::ToastOverlay,
    names: &Rc<std::cell::RefCell<Vec<String>>>,
    reload: &Reload,
) -> gtk::Box {
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Add a printer"))
        .build();
    let search = ndl::ActionRow::builder()
        .title(tr("Look for printers"))
        .subtitle(tr(
            "Printers on your network that can be used without a driver",
        ))
        .build();
    search.add_prefix(&gtk::Image::from_icon_name("system-search-symbolic"));
    let button = gtk::Button::with_label(tr("Search"));
    button.set_valign(gtk::Align::Center);
    search.add_suffix(&button);
    group.add(&search);

    let found_rows: Rc<std::cell::RefCell<Vec<ndl::ActionRow>>> = Rc::default();
    {
        let (group, toasts, names, reload) =
            (group.clone(), toasts.clone(), names.clone(), reload.clone());
        let search = search.clone();
        button.connect_clicked(move |button| {
            button.set_sensitive(false);
            let (button, group, toasts, names, reload, found_rows, search) = (
                button.clone(),
                group.clone(),
                toasts.clone(),
                names.clone(),
                reload.clone(),
                found_rows.clone(),
                search.clone(),
            );
            glib::spawn_future_local(async move {
                for old in found_rows.borrow_mut().drain(..) {
                    group.remove(&old);
                }
                let output = run(&[
                    "ippfind",
                    "-T",
                    "4",
                    "_ipp._tcp",
                    "-x",
                    "echo",
                    "{service_name}|{}",
                    ";",
                ])
                .await;
                button.set_sensitive(true);
                let list = output.map(|o| printers::parse_found(&o));
                let Some(list) = list else {
                    search.set_subtitle(tr("Searching is not available on this system"));
                    return;
                };
                search.set_subtitle(tr(if list.is_empty() {
                    "No printers found"
                } else {
                    "Printers on your network that can be used without a driver"
                }));
                let mut rows = Vec::new();
                for printer in list {
                    let row = ndl::ActionRow::builder().title(&printer.name).build();
                    row.set_subtitle(&printer.uri);
                    row.add_prefix(&gtk::Image::from_icon_name("printer-symbolic"));
                    let add = gtk::Button::with_label(tr("Add"));
                    add.set_valign(gtk::Align::Center);
                    add.add_css_class("suggested-action");
                    let (toasts, names, reload) = (toasts.clone(), names.clone(), reload.clone());
                    add.connect_clicked(move |add| {
                        add.set_sensitive(false);
                        let (add, printer, toasts, names, reload) = (
                            add.clone(),
                            printer.clone(),
                            toasts.clone(),
                            names.clone(),
                            reload.clone(),
                        );
                        glib::spawn_future_local(async move {
                            add_printer(&printer.name, &printer.uri, &names, &toasts, &reload)
                                .await;
                            add.set_sensitive(true);
                        });
                    });
                    row.add_suffix(&add);
                    group.add(&row);
                    rows.push(row);
                }
                *found_rows.borrow_mut() = rows;
            });
        });
    }

    let address = ndl::EntryRow::builder()
        .title(tr("Address (for example 192.168.1.20)"))
        .build();
    address.add_prefix(&gtk::Image::from_icon_name("network-wired-symbolic"));
    let add = gtk::Button::with_label(tr("Add"));
    add.set_valign(gtk::Align::Center);
    add.set_sensitive(false);
    address.add_suffix(&add);
    {
        let add = add.clone();
        address.connect_changed(move |row| {
            add.set_sensitive(printers::normalize_address(&row.text()).is_some());
        });
    }
    {
        let (address, toasts, names, reload) = (
            address.clone(),
            toasts.clone(),
            names.clone(),
            reload.clone(),
        );
        add.connect_clicked(move |add| {
            let Some(uri) = printers::normalize_address(&address.text()) else {
                return;
            };
            add.set_sensitive(false);
            let (add, address, toasts, names, reload) = (
                add.clone(),
                address.clone(),
                toasts.clone(),
                names.clone(),
                reload.clone(),
            );
            glib::spawn_future_local(async move {
                let title = address.text().trim().to_owned();
                add_printer(&title, &uri, &names, &toasts, &reload).await;
                add.set_sensitive(true);
            });
        });
    }
    let by_address = ndl::PreferencesGroup::new();
    by_address.add(&address);
    let both = gtk::Box::new(gtk::Orientation::Vertical, 18);
    both.append(&group);
    both.append(&by_address);
    both
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Printers"))
        .build();
    content.append(&group);
    let names: Rc<std::cell::RefCell<Vec<String>>> = Rc::default();

    let rows_now: Rc<std::cell::RefCell<Vec<gtk::Widget>>> = Rc::default();
    let reload: Rc<std::cell::RefCell<Option<Reload>>> = Rc::default();
    let load: Reload = {
        let (group, rows_now, reload, names) = (
            group.clone(),
            rows_now.clone(),
            reload.clone(),
            names.clone(),
        );
        let toasts = toasts.clone();
        Rc::new(move || {
            let (group, rows_now, reload, toasts, names) = (
                group.clone(),
                rows_now.clone(),
                reload.clone(),
                toasts.clone(),
                names.clone(),
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
                *names.borrow_mut() = found.iter().map(|p| p.name.clone()).collect();
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
    content.append(&add_group(toasts, &names, &load));
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
