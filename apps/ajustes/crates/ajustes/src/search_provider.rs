//! The search provider the Shell asks when the user types in the overview: it finds Settings'
//! pages (`org.gnome.Shell.SearchProvider2`, on the name and path the GNOME Settings one had, so
//! the Shell's provider file keeps working).
//!
//! It runs as its own process, `nodalix-settings --search-provider`, started by D-Bus when the
//! Shell first asks, and quits when nobody has asked for a while. It does not start GTK: it
//! needs no display, so answering costs a few megabytes. Opening a result starts
//! `nodalix-settings --page <page>`, which goes to the window that is open or opens one.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use ajustes_core::{
    pages::{self, Page},
    search,
};
use nodalia_gtk::gtk::{
    gio::{self, prelude::*},
    glib,
};

use crate::i18n::tr;

const BUS_NAME: &str = "org.gnome.Settings.SearchProvider";
const OBJECT_PATH: &str = "/org/gnome/Settings/SearchProvider";
/// A search provider that has not been asked anything for this long quits.
const IDLE_SECONDS: u32 = 45;

const INTERFACE: &str = r#"
<node>
  <interface name="org.gnome.Shell.SearchProvider2">
    <method name="GetInitialResultSet">
      <arg type="as" name="terms" direction="in"/>
      <arg type="as" name="results" direction="out"/>
    </method>
    <method name="GetSubsearchResultSet">
      <arg type="as" name="previous_results" direction="in"/>
      <arg type="as" name="terms" direction="in"/>
      <arg type="as" name="results" direction="out"/>
    </method>
    <method name="GetResultMetas">
      <arg type="as" name="identifiers" direction="in"/>
      <arg type="aa{sv}" name="metas" direction="out"/>
    </method>
    <method name="ActivateResult">
      <arg type="s" name="identifier" direction="in"/>
      <arg type="as" name="terms" direction="in"/>
      <arg type="u" name="timestamp" direction="in"/>
    </method>
    <method name="LaunchSearch">
      <arg type="as" name="terms" direction="in"/>
      <arg type="u" name="timestamp" direction="in"/>
    </method>
  </interface>
</node>"#;

/// The ids of the pages a search finds.
fn results<S: AsRef<str>>(terms: &[S]) -> Vec<String> {
    search::find(terms)
        .into_iter()
        .map(search::result_id)
        .collect()
}

/// What the Shell shows for one result: its name, the section it is in and its icon.
fn meta(page: &Page) -> HashMap<String, glib::Variant> {
    let section = pages::SECTIONS
        .iter()
        .find(|(_, pages)| pages.iter().any(|p| p.id == page.id))
        .map(|(title, _)| *title)
        .filter(|title| !title.is_empty())
        .unwrap_or("Settings");
    let mut meta = HashMap::new();
    meta.insert("id".to_owned(), search::result_id(page).to_variant());
    meta.insert("name".to_owned(), tr(page.title).to_variant());
    meta.insert("description".to_owned(), tr(section).to_variant());
    meta.insert("gicon".to_owned(), page.icon.to_variant());
    if let Some(icon) = gio::ThemedIcon::new(page.icon).serialize() {
        meta.insert("icon".to_owned(), icon);
    }
    meta
}

/// Opens Settings on a page, or on its first page.
fn open(page: Option<&Page>) {
    let program = std::env::current_exe().unwrap_or_else(|_| "nodalix-settings".into());
    let mut command = std::process::Command::new(program);
    if let Some(page) = page {
        command.arg("--page").arg(page.id);
    }
    match command.spawn() {
        // A child that is never waited for stays a zombie after it exits.
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(error) => eprintln!("nodalix-settings: {error}"),
    }
}

/// Answers one call of the provider. `Err` is a D-Bus error message.
fn call(method: &str, parameters: &glib::Variant) -> Result<Option<glib::Variant>, String> {
    let bad = || format!("{method}: unexpected arguments");
    match method {
        "GetInitialResultSet" => {
            let (terms,) = parameters.get::<(Vec<String>,)>().ok_or_else(bad)?;
            Ok(Some((results(&terms),).to_variant()))
        }
        "GetSubsearchResultSet" => {
            let (previous, terms) = parameters
                .get::<(Vec<String>, Vec<String>)>()
                .ok_or_else(bad)?;
            // What the new terms find, among what the last search found.
            let found = results(&terms);
            let narrowed: Vec<String> = found
                .into_iter()
                .filter(|id| previous.contains(id))
                .collect();
            Ok(Some((narrowed,).to_variant()))
        }
        "GetResultMetas" => {
            let (ids,) = parameters.get::<(Vec<String>,)>().ok_or_else(bad)?;
            let metas: Vec<_> = ids
                .iter()
                .filter_map(|id| search::page_of(id))
                .map(meta)
                .collect();
            Ok(Some((metas,).to_variant()))
        }
        "ActivateResult" => {
            let (id, _terms, _time) = parameters
                .get::<(String, Vec<String>, u32)>()
                .ok_or_else(bad)?;
            open(search::page_of(&id));
            Ok(None)
        }
        "LaunchSearch" => {
            let (terms, _time) = parameters.get::<(Vec<String>, u32)>().ok_or_else(bad)?;
            open(search::find(&terms).first().copied());
            Ok(None)
        }
        other => Err(format!("unknown method {other}")),
    }
}

/// Runs the provider until it has been idle for a while.
pub fn run() -> glib::ExitCode {
    // Not unique: a Settings window that is open is another process, and this one must not hand
    // its command line to it.
    let app = gio::Application::new(
        Some("os.nodalix.Settings.Search"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    app.connect_activate(|app| {
        let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
        // Every call starts the countdown again.
        let touch: Rc<dyn Fn()> = {
            let (app, timer) = (app.clone(), timer.clone());
            Rc::new(move || {
                if let Some(old) = timer.borrow_mut().take() {
                    old.remove();
                }
                let (app, slot) = (app.clone(), timer.clone());
                *timer.borrow_mut() = Some(glib::timeout_add_seconds_local_once(
                    IDLE_SECONDS,
                    move || {
                        slot.borrow_mut().take();
                        app.quit();
                    },
                ));
            })
        };
        touch();
        // The loop runs while the app is held.
        std::mem::forget(app.hold());

        let node = gio::DBusNodeInfo::for_xml(INTERFACE).expect("the interface is valid XML");
        let interface = node
            .lookup_interface("org.gnome.Shell.SearchProvider2")
            .expect("the interface is in the XML");
        let (on_call, quit) = (touch.clone(), app.clone());
        gio::bus_own_name(
            gio::BusType::Session,
            BUS_NAME,
            gio::BusNameOwnerFlags::NONE,
            move |connection, _| {
                let touch = on_call.clone();
                let registered = connection
                    .register_object(OBJECT_PATH, &interface)
                    .method_call(move |_, _, _, _, method, parameters, invocation| {
                        touch();
                        match call(method, &parameters) {
                            Ok(reply) => invocation.return_value(reply.as_ref()),
                            Err(message) => invocation.return_dbus_error(
                                "org.freedesktop.DBus.Error.InvalidArgs",
                                &message,
                            ),
                        }
                    })
                    .build();
                if let Err(error) = registered {
                    eprintln!("nodalix-settings: the search provider is not registered: {error}");
                }
            },
            |_, _| {},
            // Someone else has the name: nothing to do here.
            move |_, _| quit.quit(),
        );
    });
    app.run_with_args(&["nodalix-settings"])
}
