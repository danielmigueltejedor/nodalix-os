//! Users: this account's name, picture and type, from AccountsService. Adding or removing
//! accounts and changing the password stay in the previous panel.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{glib, prelude::*},
};

use super::{bus, column, gs, scroller};
use crate::i18n::tr;

const ACCOUNTS: &str = "org.freedesktop.Accounts";
const ACCOUNTS_PATH: &str = "/org/freedesktop/Accounts";
const USER: &str = "org.freedesktop.Accounts.User";

/// "Administrator" or "Standard", from AccountsService's `AccountType`.
fn account_type(kind: i32) -> &'static str {
    if kind == 1 {
        "Administrator"
    } else {
        "Standard"
    }
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Your account"))
        .build();

    let picture = gtk::Image::from_icon_name("avatar-default-symbolic");
    picture.set_pixel_size(64);
    picture.set_halign(gtk::Align::Center);
    picture.set_margin_bottom(6);
    content.append(&picture);

    let name = ndl::EntryRow::builder().title(tr("Full name")).build();
    let apply = gtk::Button::with_label(tr("Apply"));
    apply.set_valign(gtk::Align::Center);
    apply.set_sensitive(false);
    name.add_suffix(&apply);
    name.set_sensitive(false);
    let username = ndl::ActionRow::builder().title(tr("User name")).build();
    let kind = ndl::ActionRow::builder().title(tr("Account type")).build();
    group.add(&name);
    group.add(&username);
    group.add(&kind);
    content.append(&group);
    if let Some(more) = gs::group(
        "",
        crate::rows![Some(gs::legacy_link(
            tr("Password and other accounts"),
            tr("Change the password, add or remove accounts"),
            "users",
        ))],
    ) {
        content.append(&more);
    }

    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let Some(connection) = bus::system().await else {
            return;
        };
        let found = bus::call(
            &connection,
            ACCOUNTS,
            ACCOUNTS_PATH,
            ACCOUNTS,
            "FindUserByName",
            glib::user_name()
                .to_str()
                .map(|n| (n,).to_variant())
                .as_ref(),
        )
        .await
        .ok()
        .and_then(|r| r.get::<(glib::variant::ObjectPath,)>())
        .map(|(p,)| p.to_string());
        let Some(path) = found else { return };
        let Some(all) = bus::properties(&connection, ACCOUNTS, &path, USER).await else {
            return;
        };
        let text = |key: &str| {
            all.get(key)
                .and_then(|v| v.str())
                .unwrap_or_default()
                .to_owned()
        };
        let real = text("RealName");
        name.set_text(&real);
        name.set_sensitive(true);
        username.set_subtitle(&text("UserName"));
        let account = all
            .get("AccountType")
            .and_then(|v| v.get::<i32>())
            .unwrap_or(0);
        kind.set_subtitle(tr(account_type(account)));
        let icon = text("IconFile");
        if !icon.is_empty() && std::path::Path::new(&icon).is_file() {
            picture.set_from_file(Some(&icon));
        }

        {
            let changed_apply = apply.clone();
            let saved = std::rc::Rc::new(std::cell::RefCell::new(real));
            let saved2 = saved.clone();
            name.connect_changed(move |row| {
                changed_apply.set_sensitive(row.text().trim() != *saved2.borrow());
            });
            let (name, apply) = (name.clone(), apply.clone());
            apply.clone().connect_clicked(move |_| {
                let wanted = name.text().trim().to_string();
                let (connection, path, toasts, apply, saved) = (
                    connection.clone(),
                    path.clone(),
                    toasts.clone(),
                    apply.clone(),
                    saved.clone(),
                );
                glib::spawn_future_local(async move {
                    match bus::call(
                        &connection,
                        ACCOUNTS,
                        &path,
                        USER,
                        "SetRealName",
                        Some(&(wanted.as_str(),).to_variant()),
                    )
                    .await
                    {
                        Ok(_) => {
                            *saved.borrow_mut() = wanted;
                            apply.set_sensitive(false);
                        }
                        Err(error) => toasts.add_toast(ndl::Toast::new(error.message())),
                    }
                });
            });
        }
    });
    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_account_type_is_named_from_the_number() {
        assert_eq!(account_type(1), "Administrator");
        assert_eq!(account_type(0), "Standard");
        assert_eq!(account_type(7), "Standard");
    }
}
