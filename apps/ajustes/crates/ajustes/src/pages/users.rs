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

    // The picture sits in a circle: the box clips whatever it holds to its rounded shape.
    let picture = gtk::Box::new(gtk::Orientation::Vertical, 0);
    picture.add_css_class("settings-avatar");
    picture.set_overflow(gtk::Overflow::Hidden);
    picture.set_halign(gtk::Align::Center);
    picture.set_valign(gtk::Align::Start);
    picture.set_vexpand(false);
    picture.set_margin_bottom(6);
    picture.set_size_request(96, 96);
    let glyph = gtk::Image::from_icon_name("avatar-default-symbolic");
    glyph.set_pixel_size(48);
    glyph.set_halign(gtk::Align::Center);
    glyph.set_valign(gtk::Align::Center);
    glyph.set_hexpand(true);
    glyph.set_vexpand(true);
    picture.append(&glyph);
    content.append(&picture);

    let name = ndl::EntryRow::builder().title(tr("Full name")).build();
    name.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));
    let apply = gtk::Button::with_label(tr("Apply"));
    apply.set_valign(gtk::Align::Center);
    apply.set_sensitive(false);
    name.add_suffix(&apply);
    name.set_sensitive(false);
    let username = ndl::ActionRow::builder().title(tr("User name")).build();
    username.add_prefix(&gtk::Image::from_icon_name("system-users-symbolic"));
    let kind = ndl::ActionRow::builder().title(tr("Account type")).build();
    kind.add_prefix(&gtk::Image::from_icon_name("security-high-symbolic"));
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
            // A picture's natural size is the photo's own, so it goes in a scrolled window that
            // does not pass that size on: the circle stays 96 px whatever the file measures.
            let photo = gtk::Picture::for_filename(&icon);
            photo.set_content_fit(gtk::ContentFit::Cover);
            photo.set_can_shrink(true);
            photo.set_hexpand(true);
            photo.set_vexpand(true);
            let fixed = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .min_content_width(96)
                .min_content_height(96)
                .max_content_width(96)
                .max_content_height(96)
                .propagate_natural_width(false)
                .propagate_natural_height(false)
                .child(&photo)
                .build();
            picture.remove(&glyph);
            picture.append(&fixed);
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
