//! Cloud storage: Google Drive, OneDrive, Dropbox and iCloud Drive, set up with `rclone` and
//! mounted under `~/Nube` so every program, the file manager first, sees them as folders. The
//! list of accounts is a file the file manager reads; a mount is a user service per account.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::cloud::{self, Account, Provider, Step};

use super::cmd::{capture, run};
use super::{column, scroller};
use crate::i18n::tr;

const POLL_SECONDS: u32 = 5;

fn home() -> String {
    glib::home_dir().to_string_lossy().into_owned()
}

fn list_file() -> PathBuf {
    glib::user_config_dir().join("nodalix").join("cloud.json")
}

fn read_accounts() -> Vec<Account> {
    std::fs::read_to_string(list_file())
        .map(|text| cloud::parse(&text))
        .unwrap_or_default()
}

/// Writes the list whole into a temporary file and renames it, so the file manager never reads
/// half of it.
fn write_accounts(accounts: &[Account]) -> std::io::Result<()> {
    let file = list_file();
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let temporary = file.with_extension("json.tmp");
    std::fs::write(&temporary, cloud::to_json(accounts))?;
    std::fs::rename(temporary, file)
}

fn env_file(id: &str) -> PathBuf {
    glib::user_config_dir()
        .join("nodalix")
        .join("cloud")
        .join(format!("{id}.env"))
}

/// Puts in place what an account's mount needs: the instance's folder and, once, the unit every
/// instance is made from. Returns whether the unit changed, so systemd must read it again.
fn install(account: &Account) -> std::io::Result<bool> {
    let env = env_file(&account.id);
    if let Some(folder) = env.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(env, cloud::instance_env(&account.mount_dir(&home())))?;
    let unit = glib::user_config_dir()
        .join("systemd")
        .join("user")
        .join("nodalix-cloud@.service");
    if std::fs::read_to_string(&unit).is_ok_and(|text| text == cloud::UNIT_TEMPLATE) {
        return Ok(false);
    }
    if let Some(folder) = unit.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(unit, cloud::UNIT_TEMPLATE)?;
    Ok(true)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Mounted,
    Mounting,
    Failed,
    Stopped,
}

async fn status(account: &Account) -> Status {
    let output = capture(&["systemctl", "--user", "is-active", &account.unit()], None).await;
    match output.out.trim() {
        "active" => Status::Mounted,
        "activating" | "reloading" => Status::Mounting,
        "failed" => Status::Failed,
        _ => Status::Stopped,
    }
}

fn status_text(status: Status) -> &'static str {
    match status {
        Status::Mounted => "Connected",
        Status::Mounting => "Connecting…",
        Status::Failed => "Could not connect",
        Status::Stopped => "Not connected",
    }
}

fn provider_icon(provider: Provider) -> &'static str {
    match provider {
        Provider::ICloudDrive => "weather-overcast-symbolic",
        _ => "folder-remote-symbolic",
    }
}

/// The first line a failed command said, the part worth showing in a notice.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default()
        .chars()
        .take(160)
        .collect()
}

struct Form {
    provider: ndl::ComboRow,
    name: ndl::EntryRow,
    apple_id: ndl::EntryRow,
    password: ndl::PasswordEntryRow,
    code: ndl::EntryRow,
    go: gtk::Button,
    cancel: gtk::Button,
    progress: ndl::ActionRow,
    /// The browser sign-in that is running, so Cancel can end it.
    running: RefCell<Option<gio::Subprocess>>,
    cancelled: Cell<bool>,
    /// The answer to a question for the person: `Some(None)` is a cancel.
    answer: RefCell<Option<Option<String>>>,
}

impl Form {
    fn provider(&self) -> Provider {
        Provider::ALL
            .get(self.provider.selected() as usize)
            .copied()
            .unwrap_or(Provider::Drive)
    }

    fn say(&self, text: &str) {
        self.progress.set_subtitle(text);
    }

    fn set_busy(&self, busy: bool) {
        for row in [
            self.provider.clone().upcast::<gtk::Widget>(),
            self.name.clone().upcast(),
            self.apple_id.clone().upcast(),
            self.password.clone().upcast(),
        ] {
            row.set_sensitive(!busy);
        }
        self.go.set_sensitive(!busy);
        self.cancel.set_visible(busy);
        if !busy {
            self.code.set_visible(false);
            self.code.set_text("");
        }
    }

    fn show_provider(&self) {
        let icloud = !self.provider().uses_browser();
        self.apple_id.set_visible(icloud);
        self.password.set_visible(icloud);
    }

    /// Waits for the person to give the code, or to cancel.
    async fn ask_code(&self) -> Option<String> {
        *self.answer.borrow_mut() = None;
        self.code.set_visible(true);
        self.code.grab_focus();
        self.say(tr(
            "Allow access on your Apple device and enter the six-digit code it shows",
        ));
        self.go.set_label(tr("Continue"));
        self.go.set_sensitive(true);
        loop {
            if let Some(answer) = self.answer.borrow_mut().take() {
                self.go.set_label(tr("Sign in and add"));
                self.go.set_sensitive(false);
                self.code.set_visible(false);
                return answer;
            }
            glib::timeout_future(Duration::from_millis(200)).await;
        }
    }
}

struct Page {
    toasts: ndl::ToastOverlay,
    banner: ndl::PreferencesGroup,
    list: ndl::PreferencesGroup,
    rows: RefCell<Vec<ndl::ActionRow>>,
    reloading: Cell<bool>,
    /// Looked for until it is found, so a page that polls does not start `rclone` each time.
    has_rclone: Cell<bool>,
    widget: gtk::ScrolledWindow,
}

impl Page {
    fn toast(&self, text: &str) {
        self.toasts.add_toast(ndl::Toast::new(text));
    }

    async fn reload(self: &Rc<Self>) {
        if self.reloading.replace(true) {
            return;
        }
        if !self.has_rclone.get() {
            self.has_rclone
                .set(run(&["rclone", "version"]).await.is_some());
            self.banner.set_visible(!self.has_rclone.get());
        }
        let accounts = read_accounts();
        let mut shown = Vec::new();
        for account in accounts {
            let state = status(&account).await;
            shown.push((account, state));
        }
        for old in self.rows.borrow_mut().drain(..) {
            self.list.remove(&old);
        }
        let mut rows = Vec::new();
        if shown.is_empty() {
            let row = ndl::ActionRow::builder()
                .title(tr("No cloud storage yet"))
                .subtitle(tr("Add one below and it shows up in the file manager"))
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("folder-remote-symbolic"));
            self.list.add(&row);
            rows.push(row);
        }
        for (account, state) in shown {
            let row = self.account_row(account, state);
            self.list.add(&row);
            rows.push(row);
        }
        *self.rows.borrow_mut() = rows;
        self.reloading.set(false);
    }

    fn account_row(self: &Rc<Self>, account: Account, state: Status) -> ndl::ActionRow {
        let row = ndl::ActionRow::builder()
            .title(&account.name)
            .subtitle(format!(
                "{} · {}",
                account.provider.title(),
                tr(status_text(state))
            ))
            .build();
        if account.name.trim().is_empty() {
            row.set_title(account.provider.title());
        }
        row.add_prefix(&gtk::Image::from_icon_name(provider_icon(account.provider)));
        let on = matches!(state, Status::Mounted | Status::Mounting);
        let toggle = gtk::Button::with_label(tr(if on { "Disconnect" } else { "Connect" }));
        toggle.set_valign(gtk::Align::Center);
        let remove = gtk::Button::from_icon_name("user-trash-symbolic");
        remove.set_valign(gtk::Align::Center);
        remove.set_tooltip_text(Some(tr("Remove")));
        remove.add_css_class("flat");
        row.add_suffix(&toggle);
        row.add_suffix(&remove);
        {
            let (page, account) = (self.clone(), account.clone());
            toggle.connect_clicked(move |button| {
                button.set_sensitive(false);
                let (page, account) = (page.clone(), account.clone());
                glib::spawn_future_local(async move {
                    page.set_mounted(&account, !on).await;
                });
            });
        }
        {
            let page = self.clone();
            remove.connect_clicked(move |button| {
                let (page, account) = (page.clone(), account.clone());
                let window = button.root().and_downcast::<gtk::Window>();
                glib::spawn_future_local(async move {
                    let dialog = gtk::AlertDialog::builder()
                        .message(tr("Remove this storage?"))
                        .detail(tr(
                            "It is disconnected and forgotten here. Your files in the cloud stay where they are.",
                        ))
                        .buttons([tr("Cancel"), tr("Remove")])
                        .cancel_button(0)
                        .default_button(0)
                        .modal(true)
                        .build();
                    if dialog.choose_future(window.as_ref()).await == Ok(1) {
                        page.remove(&account).await;
                    }
                });
            });
        }
        row
    }

    /// Connects or disconnects. A connected account is also set to connect when the person signs
    /// in, which is what `enable` does.
    async fn set_mounted(self: &Rc<Self>, account: &Account, on: bool) {
        match install(account) {
            Ok(true) => {
                capture(&["systemctl", "--user", "daemon-reload"], None).await;
            }
            Ok(false) => {}
            Err(_) => {
                self.toast(tr("Could not prepare the connection"));
                self.reload().await;
                return;
            }
        }
        let verb = if on { "enable" } else { "disable" };
        let output = capture(
            &["systemctl", "--user", verb, "--now", &account.unit()],
            None,
        )
        .await;
        if !output.ok {
            let detail = first_line(&output.err);
            self.toast(&if detail.is_empty() {
                tr("Could not change the connection").to_owned()
            } else {
                format!("{}: {detail}", tr("Could not change the connection"))
            });
        }
        self.reload().await;
    }

    async fn remove(self: &Rc<Self>, account: &Account) {
        capture(
            &["systemctl", "--user", "disable", "--now", &account.unit()],
            None,
        )
        .await;
        capture(&["rclone", "config", "delete", &account.remote()], None).await;
        let id = account.id.clone();
        let saved = gio::spawn_blocking(move || {
            let _ = std::fs::remove_file(env_file(&id));
            let rest: Vec<Account> = read_accounts().into_iter().filter(|a| a.id != id).collect();
            write_accounts(&rest).is_ok()
        })
        .await
        .unwrap_or(false);
        if !saved {
            self.toast(tr("Could not save the list"));
        }
        self.reload().await;
    }

    /// The whole of adding: sign in, make the `rclone` remote, record the account.
    async fn add(self: &Rc<Self>, form: &Rc<Form>) {
        form.cancelled.set(false);
        form.set_busy(true);
        let result = self.add_inner(form).await;
        form.set_busy(false);
        match result {
            Ok(()) => {
                form.say("");
                form.name.set_text("");
                form.apple_id.set_text("");
                form.password.set_text("");
                self.toast(tr("Cloud storage added"));
            }
            Err(message) => form.say(&message),
        }
        self.reload().await;
    }

    async fn add_inner(self: &Rc<Self>, form: &Rc<Form>) -> Result<(), String> {
        let provider = form.provider();
        let name = form.name.text().trim().to_owned();
        let apple_id = form.apple_id.text().trim().to_owned();
        let password = form.password.text().to_string();
        if !provider.uses_browser() && (apple_id.is_empty() || password.is_empty()) {
            return Err(tr("Enter your Apple ID and password").to_owned());
        }
        form.say(tr("Checking rclone…"));
        if run(&["rclone", "version"]).await.is_none() {
            return Err(tr("rclone is not installed").to_owned());
        }
        let taken: Vec<String> = read_accounts().into_iter().map(|a| a.id).collect();
        let account = Account {
            id: cloud::new_id(provider, &name, &taken),
            provider,
            name,
        };
        let remote = account.remote();

        let mut create: Vec<String> = ["rclone", "config", "create", &remote, provider.key()]
            .map(str::to_owned)
            .to_vec();
        if provider.uses_browser() {
            form.say(tr("Sign in in the browser window that opens…"));
            let token = authorize(form, provider).await?;
            create.extend(["token".to_owned(), token]);
        } else {
            // The password goes in obscured, as `rclone` stores it, and never on a command line
            // as it was typed.
            let obscured = capture(&["rclone", "obscure", "-"], Some(&password)).await;
            if !obscured.ok {
                return Err(tr("The password could not be prepared").to_owned());
            }
            create.extend([
                "apple_id".to_owned(),
                apple_id,
                "password".to_owned(),
                obscured.out.trim().to_owned(),
            ]);
        }
        create.push("--non-interactive".to_owned());

        form.say(tr("Setting up the connection…"));
        let created = self.configure(form, &remote, &create).await;
        if let Err(message) = created {
            capture(&["rclone", "config", "delete", &remote], None).await;
            return Err(message);
        }
        let finish = || -> std::io::Result<()> {
            install(&account)?;
            let mut accounts = read_accounts();
            accounts.push(account.clone());
            write_accounts(&accounts)
        };
        if finish().is_err() {
            capture(&["rclone", "config", "delete", &remote], None).await;
            return Err(tr("Could not save the list").to_owned());
        }
        capture(&["systemctl", "--user", "daemon-reload"], None).await;
        Ok(())
    }

    /// Runs `rclone config create` and answers what it asks: with its own default, except for a
    /// verification code, which only the person has.
    async fn configure(
        &self,
        form: &Rc<Form>,
        remote: &str,
        create: &[String],
    ) -> Result<(), String> {
        let args: Vec<&str> = create.iter().map(String::as_str).collect();
        let mut output = capture(&args, None).await;
        for _ in 0..12 {
            if !output.ok {
                let detail = first_line(&output.err);
                return Err(if detail.is_empty() {
                    tr("The connection could not be set up").to_owned()
                } else {
                    detail
                });
            }
            match cloud::parse_step(&output.out) {
                Step::Done => return Ok(()),
                Step::Failed(error) => return Err(first_line(&error)),
                Step::Ask {
                    state,
                    name,
                    default,
                    ..
                } => {
                    let answer = if cloud::asks_for_code(&name) {
                        // Ask again until it looks like a code, or the person gives up.
                        loop {
                            match form.ask_code().await {
                                Some(code) if cloud::is_valid_code(&code) => {
                                    break code.trim().to_owned()
                                }
                                Some(_) => form.say(tr("The code has six digits")),
                                None => return Err(tr("Cancelled").to_owned()),
                            }
                        }
                    } else {
                        default
                    };
                    output = capture(
                        &[
                            "rclone",
                            "config",
                            "update",
                            remote,
                            "--continue",
                            "--state",
                            &state,
                            "--result",
                            &answer,
                            "--non-interactive",
                        ],
                        None,
                    )
                    .await;
                }
            }
        }
        Err(tr("The connection could not be set up").to_owned())
    }
}

/// The browser sign-in: `rclone authorize` opens the provider's page and prints a token once the
/// person has agreed.
async fn authorize(form: &Rc<Form>, provider: Provider) -> Result<String, String> {
    let process = gio::Subprocess::newv(
        &[
            std::ffi::OsStr::new("rclone"),
            std::ffi::OsStr::new("authorize"),
            std::ffi::OsStr::new(provider.key()),
        ],
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
    )
    .map_err(|_| tr("rclone is not installed").to_owned())?;
    *form.running.borrow_mut() = Some(process.clone());
    let output = process.communicate_utf8_future(None::<String>).await;
    form.running.borrow_mut().take();
    if form.cancelled.get() {
        return Err(tr("Cancelled").to_owned());
    }
    let text = output
        .ok()
        .and_then(|(out, _)| out)
        .map(|s| s.to_string())
        .unwrap_or_default();
    cloud::extract_token(&text).ok_or_else(|| tr("The sign-in did not finish").to_owned())
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();

    let banner = ndl::PreferencesGroup::new();
    let missing = ndl::ActionRow::builder()
        .title(tr("rclone is not installed"))
        .subtitle(tr("Install the rclone package to use cloud storage"))
        .build();
    missing.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
    banner.add(&missing);
    banner.set_visible(false);
    content.append(&banner);

    let list = ndl::PreferencesGroup::builder()
        .title(tr("Your storage"))
        .build();
    content.append(&list);

    let titles: Vec<&str> = Provider::ALL.iter().map(|p| p.title()).collect();
    let provider = ndl::ComboRow::builder()
        .title(tr("Service"))
        .model(&gtk::StringList::new(&titles))
        .build();
    provider.add_prefix(&gtk::Image::from_icon_name("folder-remote-symbolic"));
    let name = ndl::EntryRow::builder().title(tr("Name")).build();
    name.add_prefix(&gtk::Image::from_icon_name("document-edit-symbolic"));
    let apple_id = ndl::EntryRow::builder().title(tr("Apple ID")).build();
    apple_id.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));
    apple_id.set_visible(false);
    let password = ndl::PasswordEntryRow::builder()
        .title(tr("Password"))
        .build();
    password.set_visible(false);
    let code = ndl::EntryRow::builder()
        .title(tr("Verification code"))
        .build();
    code.set_visible(false);

    let go = gtk::Button::with_label(tr("Sign in and add"));
    go.add_css_class("suggested-action");
    go.set_valign(gtk::Align::Center);
    let cancel = gtk::Button::with_label(tr("Cancel"));
    cancel.set_valign(gtk::Align::Center);
    cancel.set_visible(false);
    let progress = ndl::ActionRow::builder()
        .title(tr(
            "It is mounted in the Nube folder of your home and shows up in the file manager",
        ))
        .build();
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.append(&cancel);
    actions.append(&go);
    progress.add_suffix(&actions);

    let add_group = ndl::PreferencesGroup::builder()
        .title(tr("Add storage"))
        .build();
    for row in [
        provider.clone().upcast::<gtk::Widget>(),
        name.clone().upcast(),
        apple_id.clone().upcast(),
        password.clone().upcast(),
        code.clone().upcast(),
    ] {
        add_group.add(&row);
    }
    let go_group = ndl::PreferencesGroup::new();
    go_group.add(&progress);
    content.append(&add_group);
    content.append(&go_group);

    let form = Rc::new(Form {
        provider,
        name,
        apple_id,
        password,
        code,
        go,
        cancel,
        progress,
        running: RefCell::new(None),
        cancelled: Cell::new(false),
        answer: RefCell::new(None),
    });
    let widget = scroller(&content);
    let page = Rc::new(Page {
        toasts: toasts.clone(),
        banner,
        list,
        rows: RefCell::new(Vec::new()),
        reloading: Cell::new(false),
        has_rclone: Cell::new(false),
        widget: widget.clone(),
    });

    {
        let form2 = form.clone();
        form.provider
            .connect_selected_notify(move |_| form2.show_provider());
    }
    {
        let (page, form2) = (page.clone(), form.clone());
        form.go.connect_clicked(move |_| {
            // While a verification code is asked for, the button gives it.
            if form2.code.is_visible() {
                *form2.answer.borrow_mut() = Some(Some(form2.code.text().to_string()));
                return;
            }
            let (page, form) = (page.clone(), form2.clone());
            glib::spawn_future_local(async move { page.add(&form).await });
        });
    }
    {
        let form2 = form.clone();
        form.cancel.connect_clicked(move |_| {
            form2.cancelled.set(true);
            if let Some(process) = form2.running.borrow().as_ref() {
                process.force_exit();
            }
            *form2.answer.borrow_mut() = Some(None);
        });
    }
    // The state of the mounts changes by itself (a network drops), so look again now and then,
    // but only while the page is on screen.
    {
        let weak = Rc::downgrade(&page);
        glib::timeout_add_seconds_local(POLL_SECONDS, move || {
            let Some(page) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if page.widget.is_mapped() {
                glib::spawn_future_local(async move { page.reload().await });
            }
            glib::ControlFlow::Continue
        });
    }
    {
        let page = page.clone();
        glib::spawn_future_local(async move { page.reload().await });
    }
    widget
}
