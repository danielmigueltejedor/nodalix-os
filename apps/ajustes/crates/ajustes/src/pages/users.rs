//! Users: this account's name, picture, type and password, and the other accounts on the
//! computer, through AccountsService. Changes that need an administrator ask polkit, which shows
//! the system's own password prompt.

use std::{cell::RefCell, rc::Rc};

use ajustes_core::users::{self, AccountKind, Problem};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{bus, cmd, column, scroller};
use crate::i18n::tr;

const ACCOUNTS: &str = "org.freedesktop.Accounts";
const ACCOUNTS_PATH: &str = "/org/freedesktop/Accounts";
const USER: &str = "org.freedesktop.Accounts.User";

/// "Administrator" or "Standard", from AccountsService's `AccountType`.
fn account_type(kind: i32) -> &'static str {
    AccountKind::from_number(kind).title()
}

/// A D-Bus error's text without the `GDBus.Error:org.…:` prefix the bindings put in front.
fn plain(message: &str) -> String {
    match message.strip_prefix("GDBus.Error:") {
        Some(rest) => rest
            .split_once(": ")
            .map_or(rest, |(_, text)| text)
            .to_owned(),
        None => message.to_owned(),
    }
}

/// What the page shares between its parts.
struct Page {
    toasts: ndl::ToastOverlay,
    connection: RefCell<Option<gio::DBusConnection>>,
    /// The object path of the signed-in account.
    me: RefCell<String>,
    others: ndl::PreferencesGroup,
    other_rows: RefCell<Vec<gtk::Widget>>,
}

impl Page {
    fn toast(&self, text: &str) {
        self.toasts.add_toast(ndl::Toast::new(text));
    }

    fn connection(&self) -> Option<gio::DBusConnection> {
        self.connection.borrow().clone()
    }

    /// Calls AccountsService, waiting for polkit if it asks the person.
    async fn call(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        parameters: Option<&glib::Variant>,
    ) -> Result<glib::Variant, String> {
        let Some(connection) = self.connection() else {
            return Err(tr("The accounts service is not available").to_owned());
        };
        bus::call_authorised(&connection, ACCOUNTS, path, interface, method, parameters)
            .await
            .map_err(|e| plain(e.message()))
    }

    /// Sets an account's password. The password is hashed here and only the hash reaches the
    /// service; it goes to `openssl` on its standard input, never on a command line.
    async fn set_password(&self, path: &str, password: &str) -> Result<(), String> {
        let output = cmd::capture(
            &["openssl", "passwd", "-6", "-stdin"],
            Some(&format!("{password}\n")),
        )
        .await;
        let hash = output.out.trim().to_owned();
        if !output.ok || !hash.starts_with("$6$") {
            return Err(tr("The password could not be prepared").to_owned());
        }
        self.call(
            path,
            USER,
            "SetPassword",
            Some(&(hash.as_str(), "").to_variant()),
        )
        .await
        .map(|_| ())
    }

    /// Removes an account, with its files, by its object path: what is done to an account that
    /// was just created and could not be given a password.
    async fn remove_by_path(&self, path: &str) {
        let Some(connection) = self.connection() else {
            return;
        };
        let uid = bus::properties(&connection, ACCOUNTS, path, USER)
            .await
            .and_then(|all| all.get("Uid").and_then(|v| v.get::<u64>()));
        if let Some(uid) = uid {
            let _ = self
                .call(
                    ACCOUNTS_PATH,
                    ACCOUNTS,
                    "DeleteUser",
                    Some(&(uid as i64, true).to_variant()),
                )
                .await;
        }
    }

    /// The other accounts on the computer: ones the person can switch to a different type or
    /// remove.
    async fn reload_others(self: &Rc<Self>) {
        let Some(connection) = self.connection() else {
            return;
        };
        let paths = bus::call(
            &connection,
            ACCOUNTS,
            ACCOUNTS_PATH,
            ACCOUNTS,
            "ListCachedUsers",
            None,
        )
        .await
        .ok()
        .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
        .map(|(p,)| p)
        .unwrap_or_default();
        let mut found = Vec::new();
        let me = glib::user_name();
        for path in paths {
            let path = path.to_string();
            let Some(all) = bus::properties(&connection, ACCOUNTS, &path, USER).await else {
                continue;
            };
            let text = |key: &str| {
                all.get(key)
                    .and_then(|v| v.str())
                    .unwrap_or_default()
                    .to_owned()
            };
            let user_name = text("UserName");
            if user_name.is_empty() || me.to_str() == Some(user_name.as_str()) {
                continue;
            }
            let kind = AccountKind::from_number(
                all.get("AccountType")
                    .and_then(|v| v.get::<i32>())
                    .unwrap_or(0),
            );
            let uid = all.get("Uid").and_then(|v| v.get::<u64>()).unwrap_or(0);
            found.push((path, user_name, text("RealName"), kind, uid));
        }
        for row in self.other_rows.borrow_mut().drain(..) {
            self.others.remove(&row);
        }
        let mut rows = Vec::new();
        if found.is_empty() {
            let row = ndl::ActionRow::builder()
                .title(tr("There are no other accounts"))
                .build();
            self.others.add(&row);
            rows.push(row.upcast::<gtk::Widget>());
        }
        for (path, user_name, real, kind, uid) in found {
            let row = self.other_row(&path, &user_name, &real, kind, uid);
            self.others.add(&row);
            rows.push(row.upcast());
        }
        *self.other_rows.borrow_mut() = rows;
    }

    fn other_row(
        self: &Rc<Self>,
        path: &str,
        user_name: &str,
        real: &str,
        kind: AccountKind,
        uid: u64,
    ) -> ndl::ActionRow {
        let title = if real.is_empty() { user_name } else { real };
        let row = ndl::ActionRow::builder()
            .title(title)
            .subtitle(format!("{user_name} · {}", tr(kind.title())))
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));
        let switch = gtk::Button::with_label(tr(match kind.other() {
            AccountKind::Administrator => "Make administrator",
            AccountKind::Standard => "Make standard",
        }));
        switch.set_valign(gtk::Align::Center);
        let remove = gtk::Button::from_icon_name("edit-delete-symbolic");
        remove.add_css_class("flat");
        remove.set_valign(gtk::Align::Center);
        remove.set_tooltip_text(Some(tr("Remove account")));
        row.add_suffix(&switch);
        row.add_suffix(&remove);
        {
            let (page, path) = (self.clone(), path.to_owned());
            switch.connect_clicked(move |button| {
                button.set_sensitive(false);
                let (page, path) = (page.clone(), path.clone());
                glib::spawn_future_local(async move {
                    let wanted = kind.other().number();
                    let result = page
                        .call(&path, USER, "SetAccountType", Some(&(wanted,).to_variant()))
                        .await;
                    if let Err(error) = result {
                        page.toast(&error);
                    }
                    page.reload_others().await;
                });
            });
        }
        {
            let (page, title) = (self.clone(), title.to_owned());
            remove.connect_clicked(move |button| {
                let window = button.root().and_downcast::<gtk::Window>();
                let (page, title) = (page.clone(), title.clone());
                glib::spawn_future_local(async move {
                    let dialog = gtk::AlertDialog::builder()
                        .message(format!("{} {title}?", tr("Remove the account of")))
                        .detail(tr(
                            "The person will no longer be able to sign in. You can keep their files or delete them too.",
                        ))
                        .buttons([tr("Cancel"), tr("Keep files"), tr("Delete files")])
                        .cancel_button(0)
                        .default_button(0)
                        .modal(true)
                        .build();
                    let choice = dialog.choose_future(window.as_ref()).await;
                    let Ok(choice @ (1 | 2)) = choice else {
                        return;
                    };
                    let result = page
                        .call(
                            ACCOUNTS_PATH,
                            ACCOUNTS,
                            "DeleteUser",
                            Some(&(uid as i64, choice == 2).to_variant()),
                        )
                        .await;
                    if let Err(error) = result {
                        page.toast(&error);
                    }
                    page.reload_others().await;
                });
            });
        }
        row
    }
}

/// What the form for a new account says is wrong.
fn problem_text(problem: Problem) -> &'static str {
    match problem {
        Problem::NoUserName => "Enter a user name",
        Problem::BadUserName => {
            "The user name can only have lowercase letters, digits, - and _, and starts with a letter"
        }
        Problem::BadFullName => "The full name cannot have colons or line breaks",
        Problem::NoPassword => "Enter a password",
        Problem::PasswordsDiffer => "The passwords are not the same",
    }
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let page = Rc::new(Page {
        toasts: toasts.clone(),
        connection: RefCell::new(None),
        me: RefCell::new(String::new()),
        others: ndl::PreferencesGroup::builder()
            .title(tr("Other accounts"))
            .build(),
        other_rows: RefCell::new(Vec::new()),
    });
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

    content.append(&password_group(&page));
    content.append(&page.others);
    content.append(&new_account_group(&page));

    let page2 = page.clone();
    glib::spawn_future_local(async move {
        let page = page2;
        let Some(connection) = bus::system().await else {
            return;
        };
        *page.connection.borrow_mut() = Some(connection.clone());
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
        *page.me.borrow_mut() = path.clone();
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
            let saved = Rc::new(RefCell::new(real));
            let saved2 = saved.clone();
            name.connect_changed(move |row| {
                changed_apply.set_sensitive(row.text().trim() != *saved2.borrow());
            });
            let (name, apply) = (name.clone(), apply.clone());
            let page = page.clone();
            apply.clone().connect_clicked(move |_| {
                let wanted = name.text().trim().to_string();
                let (page, path, apply, saved) =
                    (page.clone(), path.clone(), apply.clone(), saved.clone());
                glib::spawn_future_local(async move {
                    match page
                        .call(
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
                        Err(error) => page.toast(&error),
                    }
                });
            });
        }
        page.reload_others().await;
    });
    scroller(&content)
}

/// Change this account's password: the new one, typed twice.
fn password_group(page: &Rc<Page>) -> ndl::PreferencesGroup {
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Password"))
        .description(tr("Changing it asks for an administrator's password"))
        .build();
    let first = ndl::PasswordEntryRow::builder()
        .title(tr("New password"))
        .build();
    first.add_prefix(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
    let again = ndl::PasswordEntryRow::builder()
        .title(tr("Repeat the password"))
        .build();
    again.add_prefix(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
    let button = gtk::Button::with_label(tr("Change password"));
    button.add_css_class("suggested-action");
    button.set_valign(gtk::Align::Center);
    let row = ndl::ActionRow::builder().build();
    row.add_suffix(&button);
    group.add(&first);
    group.add(&again);
    group.add(&row);
    let page = page.clone();
    button.connect_clicked(move |button| {
        let (password, repeated) = (first.text().to_string(), again.text().to_string());
        if let Err(problem) = users::check_password(&password, &repeated) {
            page.toast(tr(problem_text(problem)));
            return;
        }
        let path = page.me.borrow().clone();
        if path.is_empty() {
            page.toast(tr("The accounts service is not available"));
            return;
        }
        button.set_sensitive(false);
        let (page, button, first, again) =
            (page.clone(), button.clone(), first.clone(), again.clone());
        glib::spawn_future_local(async move {
            match page.set_password(&path, &password).await {
                Ok(()) => {
                    first.set_text("");
                    again.set_text("");
                    page.toast(tr("Password changed"));
                }
                Err(error) => page.toast(&error),
            }
            button.set_sensitive(true);
        });
    });
    group
}

/// Create another account on the computer.
fn new_account_group(page: &Rc<Page>) -> ndl::PreferencesGroup {
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Add an account"))
        .build();
    let full = ndl::EntryRow::builder().title(tr("Full name")).build();
    full.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));
    let user = ndl::EntryRow::builder().title(tr("User name")).build();
    user.add_prefix(&gtk::Image::from_icon_name("system-users-symbolic"));
    let first = ndl::PasswordEntryRow::builder()
        .title(tr("Password"))
        .build();
    first.add_prefix(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
    let again = ndl::PasswordEntryRow::builder()
        .title(tr("Repeat the password"))
        .build();
    again.add_prefix(&gtk::Image::from_icon_name("system-lock-screen-symbolic"));
    let admin = ndl::SwitchRow::builder()
        .title(tr("Administrator"))
        .subtitle(tr(
            "Can install software and change the settings of the computer",
        ))
        .build();
    admin.add_prefix(&gtk::Image::from_icon_name("security-high-symbolic"));
    let button = gtk::Button::with_label(tr("Add account"));
    button.add_css_class("suggested-action");
    button.set_valign(gtk::Align::Center);
    let row = ndl::ActionRow::builder().build();
    row.add_suffix(&button);
    for widget in [
        full.clone().upcast::<gtk::Widget>(),
        user.clone().upcast(),
        first.clone().upcast(),
        again.clone().upcast(),
        admin.clone().upcast(),
        row.upcast(),
    ] {
        group.add(&widget);
    }

    // The user name follows the full name until the person types their own.
    let own_name = Rc::new(std::cell::Cell::new(false));
    {
        let (target, followed) = (user.clone(), own_name.clone());
        full.connect_changed(move |full| {
            if !followed.get() {
                target.set_text(&users::suggest_user_name(&full.text()));
            }
        });
        let own_name = own_name.clone();
        user.connect_changed(move |user| {
            if user.focus_child().is_some() {
                own_name.set(!user.text().is_empty());
            }
        });
    }

    let page = page.clone();
    button.connect_clicked(move |button| {
        let (full_name, user_name) = (full.text().trim().to_string(), user.text().to_string());
        let (password, repeated) = (first.text().to_string(), again.text().to_string());
        if let Err(problem) = users::check_new(&user_name, &full_name, &password, &repeated) {
            page.toast(tr(problem_text(problem)));
            return;
        }
        let kind = if admin.is_active() {
            AccountKind::Administrator
        } else {
            AccountKind::Standard
        };
        button.set_sensitive(false);
        let (page, button) = (page.clone(), button.clone());
        let fields = (full.clone(), user.clone(), first.clone(), again.clone());
        let admin = admin.clone();
        glib::spawn_future_local(async move {
            let created = page
                .call(
                    ACCOUNTS_PATH,
                    ACCOUNTS,
                    "CreateUser",
                    Some(&(user_name.as_str(), full_name.as_str(), kind.number()).to_variant()),
                )
                .await;
            match created {
                Ok(reply) => {
                    let path = reply
                        .get::<(glib::variant::ObjectPath,)>()
                        .map(|(p,)| p.to_string())
                        .unwrap_or_default();
                    match page.set_password(&path, &password).await {
                        Ok(()) => {
                            fields.0.set_text("");
                            fields.1.set_text("");
                            fields.2.set_text("");
                            fields.3.set_text("");
                            admin.set_active(false);
                            page.toast(tr("Account added"));
                        }
                        Err(error) => {
                            // An account without a password cannot sign in: take it back.
                            page.remove_by_path(&path).await;
                            page.toast(&error);
                        }
                    }
                }
                Err(error) => page.toast(&error),
            }
            page.reload_others().await;
            button.set_sensitive(true);
        });
    });
    group
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_error_loses_its_prefix() {
        assert_eq!(
            plain("GDBus.Error:org.freedesktop.Accounts.Error.PermissionDenied: Not authorized"),
            "Not authorized"
        );
        assert_eq!(plain("plain text"), "plain text");
    }

    #[test]
    fn the_account_type_is_named_from_the_number() {
        assert_eq!(account_type(1), "Administrator");
        assert_eq!(account_type(0), "Standard");
        assert_eq!(account_type(7), "Standard");
    }
}
