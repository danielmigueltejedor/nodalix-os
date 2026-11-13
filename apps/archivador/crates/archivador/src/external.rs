//! What other programs ask of Archivador: the command line Nautilus took (folders, URIs,
//! `--new-window`, `--select`, `--quit`), and the freedesktop.org `org.freedesktop.FileManager1`
//! interface on the session bus, through which browsers, chat apps, Steam and the OpenURI portal
//! "show in folder". The name is claimed at startup, so D-Bus can start Archivador for it with
//! `--gapplication-service`, without a window, and it serves the call.

use std::path::{Path, PathBuf};

use archivador_core::launch::{self, Target};
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::browser::{Browser, RECENT, TRASH};

pub const BUS_NAME: &str = "org.freedesktop.FileManager1";
const OBJECT_PATH: &str = "/org/freedesktop/FileManager1";
const INTERFACE: &str = r#"<node>
  <interface name="org.freedesktop.FileManager1">
    <method name="ShowFolders">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItems">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItemProperties">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
  </interface>
</node>"#;

/// With `NODALIX_ARCHIVADOR_TRACE` set to a file, each window opened for another program is
/// written down there, one line each: what the tests read back.
fn trace(what: &str, folder: &Path, selected: &[PathBuf]) {
    let Some(file) = std::env::var_os("NODALIX_ARCHIVADOR_TRACE") else {
        return;
    };
    let selected: Vec<String> = selected
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let line = format!(
        "{what}\t{}\t{}\n",
        folder.to_string_lossy(),
        selected.join("\t")
    );
    use std::io::Write;
    if let Ok(mut out) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    {
        let _ = out.write_all(line.as_bytes());
    }
}

fn place(target: Target) -> PathBuf {
    match target {
        Target::Path(path) => path,
        Target::Trash => PathBuf::from(TRASH),
        Target::Recent => PathBuf::from(RECENT),
    }
}

/// Opens a window on `folder`, with `selected` selected in it.
fn open(
    app: &gtk::Application,
    folder: PathBuf,
    selected: Vec<PathBuf>,
    startup_id: &str,
) -> std::rc::Rc<Browser> {
    trace("show", &folder, &selected);
    let browser = Browser::new(app, folder);
    if !selected.is_empty() {
        browser.select_after_load(selected);
    }
    browser.set_startup_id(startup_id);
    browser.present();
    browser
}

/// Opens each folder; a file opens the folder that holds it, with the file selected.
fn show_locations(app: &gtk::Application, targets: Vec<Target>, startup_id: &str) {
    let mut items = Vec::new();
    for target in targets {
        match target {
            Target::Path(path) if !path.is_dir() => items.push(path),
            other => {
                open(app, place(other), Vec::new(), startup_id);
            }
        }
    }
    show_items(app, &items, startup_id);
}

/// Opens the folder of each item, the items selected in it.
fn show_items(app: &gtk::Application, items: &[PathBuf], startup_id: &str) {
    for (folder, selected) in launch::by_folder(items) {
        open(app, folder, selected, startup_id);
    }
}

fn paths(uris: &[String]) -> Vec<Target> {
    let root = Path::new("/");
    uris.iter()
        .filter_map(|u| launch::target(u, root))
        .collect()
}

/// Serves one call of `org.freedesktop.FileManager1`.
fn call(
    app: &gtk::Application,
    method: &str,
    uris: &[String],
    startup_id: &str,
) -> Result<(), String> {
    let targets = paths(uris);
    if targets.is_empty() && !uris.is_empty() {
        return Err(format!("None of these can be shown: {}", uris.join(", ")));
    }
    match method {
        "ShowFolders" => {
            for target in targets {
                open(app, place(target), Vec::new(), startup_id);
            }
        }
        "ShowItems" => {
            let mut items = Vec::new();
            for target in targets {
                match target {
                    Target::Path(path) => items.push(path),
                    // The trash and Recent are not in a folder: they are shown themselves.
                    other => {
                        open(app, place(other), Vec::new(), startup_id);
                    }
                }
            }
            show_items(app, &items, startup_id);
        }
        "ShowItemProperties" => {
            let items: Vec<PathBuf> = targets
                .into_iter()
                .filter_map(|t| match t {
                    Target::Path(path) => Some(path),
                    _ => None,
                })
                .collect();
            for (folder, selected) in launch::by_folder(&items) {
                trace("properties", &folder, &selected);
                let shown = if selected.is_empty() {
                    vec![folder.clone()]
                } else {
                    selected.clone()
                };
                let browser = open(app, folder, selected, startup_id);
                browser.show_properties(&shown);
            }
        }
        other => return Err(format!("No method {other}")),
    }
    Ok(())
}

/// Puts `org.freedesktop.FileManager1` on the session bus, on the application's own connection.
/// The object is there before the name is asked for, so a call that comes the moment the name is
/// ours finds it.
pub fn serve(app: &gtk::Application) {
    let Some(connection) = app.dbus_connection() else {
        return;
    };
    let info = match gio::DBusNodeInfo::for_xml(INTERFACE)
        .ok()
        .and_then(|node| node.lookup_interface(BUS_NAME))
    {
        Some(info) => info,
        None => return,
    };
    let weak = app.downgrade();
    let registered = connection
        .register_object(OBJECT_PATH, &info)
        .method_call(move |_, _, _, _, method, params, invocation| {
            let Some(app) = weak.upgrade() else {
                invocation.return_dbus_error("org.freedesktop.DBus.Error.Failed", "Quitting");
                return;
            };
            let Some((uris, startup_id)) = params.get::<(Vec<String>, String)>() else {
                invocation
                    .return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", "Expected (as s)");
                return;
            };
            // Windows are made while the call is answered; the hold keeps a service that has no
            // window yet from quitting under it.
            let _hold = app.hold();
            match call(&app, method, &uris, &startup_id) {
                Ok(()) => invocation.return_value(None),
                Err(message) => {
                    invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", &message)
                }
            }
        })
        .build();
    if let Err(error) = registered {
        glib::g_warning!("archivador", "Can't offer {BUS_NAME}: {error}");
        return;
    }
    // Owned for as long as Archivador runs; if another file manager has it, Archivador waits
    // in line and takes it when that one lets go.
    // Dropping the id does not give the name up; only `bus_unown_name` would.
    let _owner = gio::bus_own_name_on_connection(
        &connection,
        BUS_NAME,
        gio::BusNameOwnerFlags::ALLOW_REPLACEMENT,
        |_, _| {},
        |_, name| glib::g_message!("archivador", "{name} is held by another program"),
    );
}

/// Handles a command line, in the Archivador that is running (the first one started, or the one
/// a second start hands its arguments to).
pub fn command_line(app: &gtk::Application, line: &gio::ApplicationCommandLine) -> glib::ExitCode {
    let args: Vec<String> = line
        .arguments()
        .into_iter()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let command = launch::parse(&args);
    for option in &command.unknown {
        glib::g_message!("archivador", "{option} is not known, left alone");
    }
    if command.quit {
        app.quit();
        return glib::ExitCode::SUCCESS;
    }
    let cwd = line.cwd().unwrap_or_else(|| PathBuf::from("/"));
    let startup_id = line
        .platform_data()
        .map(|data| glib::VariantDict::new(Some(&data)))
        .and_then(|data| {
            ["activation-token", "desktop-startup-id"]
                .iter()
                .find_map(|key| data.lookup::<String>(key).ok().flatten())
        })
        .unwrap_or_default();
    let targets: Vec<Target> = command
        .locations
        .iter()
        .filter_map(|l| launch::target(l, &cwd))
        .collect();
    let selected: Vec<PathBuf> = command
        .select
        .iter()
        .filter_map(|s| launch::target(s, &cwd))
        .filter_map(|t| match t {
            Target::Path(path) => Some(path),
            _ => None,
        })
        .collect();
    if targets.is_empty() && selected.is_empty() {
        match app.active_window() {
            Some(window) if !command.new_window => window.present(),
            _ => {
                let browser = Browser::new(app, glib::home_dir());
                browser.set_startup_id(&startup_id);
                browser.present();
            }
        }
        return glib::ExitCode::SUCCESS;
    }
    show_locations(app, targets, &startup_id);
    show_items(app, &selected, &startup_id);
    glib::ExitCode::SUCCESS
}
