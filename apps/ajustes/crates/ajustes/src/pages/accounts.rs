//! Online accounts, from GNOME Online Accounts: the accounts that are set up, whether one needs
//! attention, and removing one. Adding an account is the provider's own sign-in, which stays in
//! the previous panel.

use std::collections::HashMap;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{glib, prelude::*},
};

use super::{bus, column, gs, scroller};
use crate::i18n::tr;
use crate::rows;

const GOA: &str = "org.gnome.OnlineAccounts";
const GOA_PATH: &str = "/org/gnome/OnlineAccounts";
const ACCOUNT: &str = "org.gnome.OnlineAccounts.Account";

#[derive(Debug, PartialEq, Eq)]
struct Account {
    path: String,
    provider: String,
    kind: String,
    identity: String,
    attention: bool,
}

type Interfaces = HashMap<String, HashMap<String, glib::Variant>>;

/// The accounts in a `GetManagedObjects` reply.
fn parse(reply: &glib::Variant) -> Vec<Account> {
    let Some((objects,)) = reply.get::<(HashMap<glib::variant::ObjectPath, Interfaces>,)>() else {
        return Vec::new();
    };
    let mut out: Vec<Account> = objects
        .into_iter()
        .filter_map(|(path, interfaces)| {
            let props = interfaces.get(ACCOUNT)?;
            let text = |key: &str| {
                props
                    .get(key)
                    .and_then(|v| v.str().map(str::to_owned))
                    .unwrap_or_default()
            };
            Some(Account {
                path: path.to_string(),
                provider: text("ProviderName"),
                kind: text("ProviderType"),
                identity: {
                    let shown = text("PresentationIdentity");
                    if shown.is_empty() {
                        text("Identity")
                    } else {
                        shown
                    }
                },
                attention: props
                    .get("AttentionNeeded")
                    .and_then(|v| v.get::<bool>())
                    .unwrap_or(false),
            })
        })
        .collect();
    out.sort_by(|a, b| (&a.provider, &a.identity).cmp(&(&b.provider, &b.identity)));
    out
}

/// An icon for a provider: GNOME's `goa-account-<type>` when the theme has one.
fn icon_for(kind: &str) -> String {
    let wanted = format!("goa-account-{kind}");
    match gtk::gdk::Display::default() {
        Some(display) if gtk::IconTheme::for_display(&display).has_icon(&wanted) => wanted,
        _ => "goa-panel-symbolic".to_owned(),
    }
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Accounts"))
        .build();
    let placeholder = ndl::ActionRow::builder()
        .title(tr("Looking for accounts…"))
        .build();
    group.add(&placeholder);
    content.append(&group);
    if let Some(more) = gs::group(
        "",
        rows![Some(gs::legacy_link(
            tr("Add an account"),
            tr("Sign in to Google, Nextcloud, Microsoft and others"),
            "online-accounts",
        ))],
    ) {
        content.append(&more);
    }

    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let shown: Rc<std::cell::RefCell<Vec<gtk::Widget>>> = Rc::default();
        shown.borrow_mut().push(placeholder.clone().upcast());
        let accounts = async {
            let connection = bus::session().await?;
            let reply = bus::call(
                &connection,
                GOA,
                GOA_PATH,
                "org.freedesktop.DBus.ObjectManager",
                "GetManagedObjects",
                None,
            )
            .await
            .ok()?;
            Some((connection, parse(&reply)))
        }
        .await;
        for old in shown.borrow_mut().drain(..) {
            group.remove(&old);
        }
        let Some((connection, accounts)) = accounts else {
            let row = ndl::ActionRow::builder()
                .title(tr("The accounts service is not available"))
                .build();
            group.add(&row);
            return;
        };
        if accounts.is_empty() {
            let row = ndl::ActionRow::builder()
                .title(tr("No online accounts"))
                .build();
            group.add(&row);
        }
        for account in accounts {
            let row = ndl::ActionRow::builder().title(&account.provider).build();
            let mut subtitle = account.identity.clone();
            if account.attention {
                subtitle.push_str(" · ");
                subtitle.push_str(tr("Sign in again"));
            }
            row.set_subtitle(&subtitle);
            row.add_prefix(&gtk::Image::from_icon_name(&icon_for(&account.kind)));
            let remove = gtk::Button::from_icon_name("user-trash-symbolic");
            remove.set_valign(gtk::Align::Center);
            remove.set_tooltip_text(Some(tr("Remove account")));
            remove.add_css_class("flat");
            row.add_suffix(&remove);
            group.add(&row);
            let (connection, toasts, group2, row2) = (
                connection.clone(),
                toasts.clone(),
                group.clone(),
                row.clone(),
            );
            remove.connect_clicked(move |button| {
                let (connection, toasts, group, row, path) = (
                    connection.clone(),
                    toasts.clone(),
                    group2.clone(),
                    row2.clone(),
                    account.path.clone(),
                );
                let window = button.root().and_downcast::<gtk::Window>();
                let title = format!("{} · {}", account.provider, account.identity);
                glib::spawn_future_local(async move {
                    let dialog = gtk::AlertDialog::builder()
                        .message(tr("Remove this account?"))
                        .detail(title)
                        .buttons([tr("Cancel"), tr("Remove")])
                        .cancel_button(0)
                        .default_button(0)
                        .modal(true)
                        .build();
                    if dialog.choose_future(window.as_ref()).await != Ok(1) {
                        return;
                    }
                    match bus::call(&connection, GOA, &path, ACCOUNT, "Remove", None).await {
                        Ok(_) => group.remove(&row),
                        Err(error) => toasts.add_toast(ndl::Toast::new(error.message())),
                    }
                });
            });
        }
    });
    scroller(&content)
}
