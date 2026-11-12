//! Region and language: the language of the interface (kept by AccountsService for this user,
//! so it applies the next time they sign in) and the formats for dates, numbers and currency
//! (`org.gnome.system.locale`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::locales::{self, Locale};

use super::{bus, column, gs, scroller};
use crate::i18n::tr;

const ACCOUNTS: &str = "org.freedesktop.Accounts";
const ACCOUNTS_PATH: &str = "/org/freedesktop/Accounts";
const USER: &str = "org.freedesktop.Accounts.User";

/// The installed locales, from `locale -a`.
async fn installed() -> Vec<Locale> {
    let Ok(process) = gio::Subprocess::newv(
        &[std::ffi::OsStr::new("locale"), std::ffi::OsStr::new("-a")],
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
    ) else {
        return Vec::new();
    };
    match process.communicate_utf8_future(None).await {
        Ok((Some(out), _)) => locales::parse(&out),
        _ => Vec::new(),
    }
}

/// The path of this user in AccountsService.
async fn user_path(connection: &gio::DBusConnection) -> Option<String> {
    let name = glib::user_name();
    let reply = bus::call(
        connection,
        ACCOUNTS,
        ACCOUNTS_PATH,
        ACCOUNTS,
        "FindUserByName",
        Some(&(name.to_str()?,).to_variant()),
    )
    .await
    .ok()?;
    reply
        .get::<(glib::variant::ObjectPath,)>()
        .map(|(p,)| p.to_string())
}

fn session_language() -> String {
    std::env::var("LANG").unwrap_or_default()
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();

    let language = ndl::ComboRow::builder().title(tr("Language")).build();
    language.set_subtitle(tr("Applies the next time you sign in"));
    language.set_sensitive(false);
    let formats = ndl::ComboRow::builder().title(tr("Formats")).build();
    formats.set_subtitle(tr("Dates, numbers and currency"));
    formats.set_sensitive(false);

    let language_group = ndl::PreferencesGroup::new();
    language_group.add(&language);
    let formats_group = ndl::PreferencesGroup::new();
    formats_group.add(&formats);
    content.append(&language_group);
    content.append(&formats_group);

    let locale_settings = gs::open("org.gnome.system.locale");
    if locale_settings.is_none() {
        formats_group.set_visible(false);
    }

    let list: Rc<RefCell<Vec<Locale>>> = Rc::default();
    let loading = Rc::new(Cell::new(true));
    let user: Rc<RefCell<Option<(gio::DBusConnection, String)>>> = Rc::default();

    {
        let (language, formats, list, loading, user) = (
            language.clone(),
            formats.clone(),
            list.clone(),
            loading.clone(),
            user.clone(),
        );
        let settings = locale_settings.clone();
        glib::spawn_future_local(async move {
            let found = installed().await;
            let labels: Vec<&str> = found.iter().map(|l| l.label.as_str()).collect();
            let none = tr("Same as the language");
            let mut with_default = vec![none];
            with_default.extend(labels.iter().copied());

            // The language comes from AccountsService; without it, the session's own.
            let mut current = session_language();
            if let Some(connection) = bus::system().await {
                if let Some(path) = user_path(&connection).await {
                    if let Some(all) = bus::properties(&connection, ACCOUNTS, &path, USER).await {
                        if let Some(lang) = all.get("Language").and_then(|v| v.str()) {
                            if !lang.is_empty() {
                                current = lang.to_owned();
                            }
                        }
                    }
                    *user.borrow_mut() = Some((connection, path));
                }
            }

            loading.set(true);
            language.set_model(Some(&gtk::StringList::new(&labels)));
            formats.set_model(Some(&gtk::StringList::new(&with_default)));
            let normalized = locales::normalize(&current).unwrap_or(current);
            if let Some(pos) = found.iter().position(|l| l.code == normalized) {
                language.set_selected(pos as u32);
            }
            if let Some(settings) = settings {
                let region = settings.settings().string("region");
                let pos = found.iter().position(|l| l.code == region.as_str());
                formats.set_selected(pos.map_or(0, |p| p as u32 + 1));
            }
            language.set_sensitive(!found.is_empty() && user.borrow().is_some());
            formats.set_sensitive(!found.is_empty());
            *list.borrow_mut() = found;
            loading.set(false);
        });
    }

    {
        let (list, loading, user, toasts) =
            (list.clone(), loading.clone(), user.clone(), toasts.clone());
        language.connect_selected_notify(move |row| {
            if loading.get() {
                return;
            }
            let Some(code) = list
                .borrow()
                .get(row.selected() as usize)
                .map(|l| l.code.clone())
            else {
                return;
            };
            let Some((connection, path)) = user.borrow().clone() else {
                return;
            };
            let toasts = toasts.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = bus::call(
                    &connection,
                    ACCOUNTS,
                    &path,
                    USER,
                    "SetLanguage",
                    Some(&(code.as_str(),).to_variant()),
                )
                .await
                {
                    toasts.add_toast(ndl::Toast::new(error.message()));
                }
            });
        });
    }

    if let Some(settings) = locale_settings {
        let (list, loading) = (list, loading);
        formats.connect_selected_notify(move |row| {
            if loading.get() {
                return;
            }
            let selected = row.selected() as usize;
            let region = if selected == 0 {
                String::new()
            } else {
                match list.borrow().get(selected - 1) {
                    Some(l) => l.code.clone(),
                    None => return,
                }
            };
            // A refused write (a locked key) is not reported: the row shows the stored value
            // again the next time the page opens.
            let _ = settings.settings().set_string("region", &region);
        });
    }

    scroller(&content)
}
