//! LocalSend, a client of the session service `com.nodalix.Settings` like Updates. The service
//! keeps the settings of the GLocalSend extension and asks the running LocalSend for its devices;
//! Files and the quick settings share the same state, so this page only shows it and changes it
//! through the service.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ajustes_core::localsend::{self as ls, Device, LocalSendState};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{column, scroller};
use crate::i18n::tr;

const BUS: &str = "com.nodalix.Settings";
const PATH: &str = "/com/nodalix/Settings";
const INTERFACE: &str = "com.nodalix.Settings1";
/// Devices come and go on the network and nothing signals it, so the page asks again.
const POLL_SECONDS: u32 = 3;

pub struct LocalSendPage {
    pub widget: gtk::ScrolledWindow,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    proxy: RefCell<Option<gio::DBusProxy>>,
    status: ndl::ActionRow,
    enabled: ndl::SwitchRow,
    search: gtk::Button,
    options: gtk::Box,
    alias: ndl::EntryRow,
    alias_apply: gtk::Button,
    folder: ndl::ActionRow,
    start_on_login: ndl::SwitchRow,
    auto_disable: ndl::SwitchRow,
    auto_accept: ndl::SwitchRow,
    minutes: ndl::SpinRow,
    devices: ndl::PreferencesGroup,
    device_rows: RefCell<Vec<ndl::ActionRow>>,
    shown_devices: RefCell<Option<Vec<Device>>>,
    /// The saved name, to know whether what is typed still needs applying.
    saved_alias: RefCell<String>,
    /// A reading is in flight.
    fetching: Cell<bool>,
    /// The widgets are being set from a reading; their change handlers must not write back.
    loading: Cell<bool>,
}

fn switch(title: &str, subtitle: &str) -> ndl::SwitchRow {
    let row = ndl::SwitchRow::builder().title(title).build();
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }
    row
}

impl LocalSendPage {
    pub fn new() -> Rc<Self> {
        let content = column();

        let share = ndl::PreferencesGroup::builder()
            .title(tr("Share files"))
            .build();
        let status = ndl::ActionRow::builder()
            .title(tr("Connecting to LocalSend…"))
            .build();
        status.add_prefix(&gtk::Image::from_icon_name("emblem-shared-symbolic"));
        share.add(&status);
        let enabled = switch(tr("LocalSend is on"), "");
        enabled.set_sensitive(false);
        share.add(&enabled);
        content.append(&share);
        let search = gtk::Button::with_label(tr("Search for devices"));
        search.set_halign(gtk::Align::Start);
        search.set_sensitive(false);
        content.append(&search);

        let config = ndl::PreferencesGroup::builder()
            .title(tr("Configuration"))
            .build();
        let alias = ndl::EntryRow::builder()
            .title(tr("Name of this computer"))
            .build();
        let alias_apply = gtk::Button::with_label(tr("Apply"));
        alias_apply.set_valign(gtk::Align::Center);
        alias_apply.set_sensitive(false);
        alias.add_suffix(&alias_apply);
        config.add(&alias);
        let folder = ndl::ActionRow::builder()
            .title(tr("Receiving folder"))
            .build();
        let choose = gtk::Button::with_label(tr("Choose…"));
        choose.set_valign(gtk::Align::Center);
        folder.add_suffix(&choose);
        config.add(&folder);
        let start_on_login = switch(tr("Start when I sign in"), "");
        let auto_disable = switch(
            tr("Switch off automatically"),
            tr("Turn this off to keep LocalSend on all the time"),
        );
        let auto_accept = switch(
            tr("Accept files automatically"),
            tr("Devices on your network can send without confirmation"),
        );
        let minutes = ndl::SpinRow::with_range(
            f64::from(*ls::MINUTES.start()),
            f64::from(*ls::MINUTES.end()),
            1.0,
        );
        minutes.set_title(tr("Minutes before switching off"));
        for row in [&start_on_login, &auto_disable, &auto_accept] {
            config.add(row);
        }
        start_on_login.add_prefix(&gtk::Image::from_icon_name("system-run-symbolic"));
        auto_disable.add_prefix(&gtk::Image::from_icon_name("alarm-symbolic"));
        auto_accept.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
        minutes.add_prefix(&gtk::Image::from_icon_name(
            "preferences-system-time-symbolic",
        ));
        alias.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
        folder.add_prefix(&gtk::Image::from_icon_name("folder-download-symbolic"));
        config.add(&minutes);
        // The whole group is one box so it can be switched off while GLocalSend is missing.
        let options = gtk::Box::new(gtk::Orientation::Vertical, 0);
        options.append(&config);
        options.set_sensitive(false);
        content.append(&options);

        let devices = ndl::PreferencesGroup::builder()
            .title(tr("Nearby devices"))
            .description(tr(
                "Star your favourites. They also come first in the file manager.",
            ))
            .build();
        content.append(&devices);

        let page = Rc::new(Self {
            widget: scroller(&content),
            toasts: RefCell::new(None),
            proxy: RefCell::new(None),
            status,
            enabled: enabled.clone(),
            search: search.clone(),
            options,
            alias: alias.clone(),
            alias_apply: alias_apply.clone(),
            folder,
            start_on_login: start_on_login.clone(),
            auto_disable: auto_disable.clone(),
            auto_accept: auto_accept.clone(),
            minutes: minutes.clone(),
            devices,
            device_rows: RefCell::new(Vec::new()),
            shown_devices: RefCell::new(None),
            saved_alias: RefCell::new(String::new()),
            fetching: Cell::new(false),
            loading: Cell::new(false),
        });

        {
            let page = page.clone();
            enabled.connect_active_notify(move |row| {
                page.call("SetLocalSendEnabled", Some((row.is_active(),).to_variant()));
            });
        }
        {
            let page = page.clone();
            search.connect_clicked(move |_| page.call("RefreshLocalSend", None));
        }
        for (row, key) in [
            (&start_on_login, ls::KEY_START_ON_LOGIN),
            (&auto_disable, ls::KEY_AUTO_DISABLE),
            (&auto_accept, ls::KEY_AUTO_ACCEPT),
        ] {
            let page = page.clone();
            row.connect_active_notify(move |row| {
                page.set_option(key, if row.is_active() { "true" } else { "false" });
            });
        }
        {
            let page = page.clone();
            minutes.connect_value_notify(move |row| {
                page.set_option(
                    ls::KEY_AUTO_DISABLE_MINUTES,
                    &(row.value() as u32).to_string(),
                );
            });
        }
        {
            let page = page.clone();
            alias.connect_changed(move |row| {
                page.alias_apply
                    .set_sensitive(!page.loading.get() && *page.saved_alias.borrow() != row.text());
            });
        }
        {
            let page = page.clone();
            alias_apply.connect_clicked(move |_| {
                let text = page.alias.text().to_string();
                page.set_option(ls::KEY_ALIAS, &ls::json_string(&text));
            });
        }
        {
            let page = page.clone();
            choose.connect_clicked(move |_| page.choose_folder());
        }
        page.connect();
        page
    }

    /// Toasts show the service's refusals; the window gives its overlay once it exists.
    pub fn set_toasts(&self, toasts: &ndl::ToastOverlay) {
        *self.toasts.borrow_mut() = Some(toasts.clone());
    }

    fn toast(&self, text: &str) {
        if let Some(t) = self.toasts.borrow().as_ref() {
            t.add_toast(ndl::Toast::new(text));
        }
    }

    fn connect(self: &Rc<Self>) {
        let page = self.clone();
        glib::spawn_future_local(async move {
            match gio::DBusProxy::for_bus_future(
                gio::BusType::Session,
                gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
                None,
                BUS,
                PATH,
                INTERFACE,
            )
            .await
            {
                Ok(proxy) => {
                    *page.proxy.borrow_mut() = Some(proxy);
                    page.refresh();
                }
                Err(error) => page.say(&error.to_string()),
            }
        });
        // One timer for the life of the page; it stops by itself once the page is gone, and
        // does nothing while the page is not on screen.
        let weak = Rc::downgrade(self);
        glib::timeout_add_seconds_local(POLL_SECONDS, move || match weak.upgrade() {
            Some(page) => {
                if page.widget.is_mapped() {
                    page.refresh();
                }
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
    }

    /// A line in the status row, for when there is no state to show.
    fn say(&self, text: &str) {
        eprintln!("com.nodalix.Settings: {text}");
        self.status.set_title(text);
    }

    fn refresh(self: &Rc<Self>) {
        let Some(proxy) = self.proxy.borrow().clone() else {
            return;
        };
        if self.fetching.replace(true) {
            return;
        }
        let page = self.clone();
        glib::spawn_future_local(async move {
            let reply = proxy
                .call_future("GetLocalSend", None, gio::DBusCallFlags::NONE, 3_000)
                .await;
            page.fetching.set(false);
            match reply {
                Ok(reply) => {
                    let json = reply
                        .child_value(0)
                        .str()
                        .map(str::to_owned)
                        .unwrap_or_default();
                    match LocalSendState::parse(&json) {
                        Ok(state) => page.render(&state),
                        Err(error) => page.say(&error.to_string()),
                    }
                }
                Err(error) => page.say(&error.to_string()),
            }
        });
    }

    fn call(self: &Rc<Self>, method: &'static str, args: Option<glib::Variant>) {
        // While the widgets are being set from a reading, a changed switch is not the user's.
        if self.loading.get() {
            return;
        }
        let Some(proxy) = self.proxy.borrow().clone() else {
            return;
        };
        let page = self.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = proxy
                .call_future(method, args.as_ref(), gio::DBusCallFlags::NONE, 3_000)
                .await
            {
                // The service words its own refusals ("Elige una carpeta de recepción existente").
                let mut error = error;
                gio::DBusError::strip_remote_error(&mut error);
                page.toast(error.message());
            }
            page.refresh();
        });
    }

    fn set_option(self: &Rc<Self>, key: &str, json: &str) {
        self.call("SetLocalSendOption", Some((key, json).to_variant()));
    }

    fn choose_folder(self: &Rc<Self>) {
        let page = self.clone();
        glib::spawn_future_local(async move {
            let dialog = gtk::FileDialog::builder()
                .title(tr("Receiving folder for LocalSend"))
                .build();
            let window = page.widget.root().and_downcast::<gtk::Window>();
            // Closing the dialog without choosing is an error to GTK, and not one to show.
            if let Ok(folder) = dialog.select_folder_future(window.as_ref()).await {
                if let Some(path) = folder.path().and_then(|p| p.to_str().map(str::to_owned)) {
                    page.set_option(ls::KEY_DOWNLOAD_FOLDER, &ls::json_string(&path));
                }
            }
        });
    }

    fn render(self: &Rc<Self>, state: &LocalSendState) {
        self.loading.set(true);
        self.status.set_title(&state.message);
        self.enabled.set_sensitive(state.ready);
        self.enabled.set_active(state.enabled);
        self.search.set_sensitive(state.can_search());
        self.options.set_sensitive(state.installed);
        if state.installed {
            let c = &state.config;
            // Do not overwrite a name that is being typed.
            if *self.saved_alias.borrow() != c.alias {
                *self.saved_alias.borrow_mut() = c.alias.clone();
                if !self.alias.text().eq(&c.alias) {
                    self.alias.set_text(&c.alias);
                }
            }
            self.alias_apply
                .set_sensitive(*self.saved_alias.borrow() != self.alias.text());
            self.folder.set_subtitle(&c.download_folder);
            self.start_on_login.set_active(c.start_on_login);
            self.auto_disable.set_active(c.auto_disable);
            self.auto_accept.set_active(c.auto_accept);
            self.minutes.set_value(f64::from(
                c.auto_disable_minutes
                    .clamp(*ls::MINUTES.start(), *ls::MINUTES.end()),
            ));
            self.minutes.set_sensitive(c.auto_disable);
        }
        self.show_devices(&state.devices);
        self.loading.set(false);
    }

    /// Rebuilds the list only when it changed, so a click is not lost to a redraw.
    fn show_devices(self: &Rc<Self>, devices: &[Device]) {
        if self.shown_devices.borrow().as_deref() == Some(devices) {
            return;
        }
        for row in self.device_rows.borrow_mut().drain(..) {
            self.devices.remove(&row);
        }
        for device in devices {
            let row = ndl::ActionRow::builder().title(&device.alias).build();
            let phone = device.model.as_deref().is_some_and(|m| {
                ["iphone", "android", "phone", "mobile"]
                    .iter()
                    .any(|p| m.to_lowercase().contains(p))
            });
            row.add_prefix(&gtk::Image::from_icon_name(if phone {
                "phone-symbolic"
            } else {
                "computer-symbolic"
            }));
            if let Some(model) = &device.model {
                row.set_subtitle(model);
            }
            let star = gtk::ToggleButton::new();
            star.set_icon_name("starred-symbolic");
            star.set_tooltip_text(Some(tr("Favourite device")));
            star.set_valign(gtk::Align::Center);
            star.set_active(device.favorite);
            let (page, fingerprint) = (self.clone(), device.fingerprint.clone());
            star.connect_toggled(move |button| {
                page.call(
                    "SetLocalSendFavorite",
                    Some((fingerprint.as_str(), button.is_active()).to_variant()),
                );
            });
            row.add_suffix(&star);
            self.devices.add(&row);
            self.device_rows.borrow_mut().push(row);
        }
        *self.shown_devices.borrow_mut() = Some(devices.to_vec());
    }
}
