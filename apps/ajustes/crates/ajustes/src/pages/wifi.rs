//! Wi-Fi, through NetworkManager's D-Bus API: switch the radio, see the networks around, connect
//! to a saved or a new one and disconnect. Enterprise networks (a login, not a password) and
//! forgetting a network stay in the previous panel.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ajustes_core::network::{self, AccessPoint};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{bus, column, scroller};
use crate::i18n::tr;

use super::nm::{
    ACCESS_POINT, CONNECTION, DEVICE, DEVICE_TYPE_WIFI, NM, NM_PATH, SETTINGS, SETTINGS_PATH,
    WIRELESS,
};
const POLL_SECONDS: u32 = 5;
/// The saved networks change rarely, so they are read again only every this many polls.
const KNOWN_EVERY: u32 = 6;

/// What one look at NetworkManager found.
struct Reading {
    enabled: bool,
    /// The Wi-Fi device's object path; none on a machine without one.
    device: Option<String>,
    /// The name of the network the device is on.
    active: Option<String>,
    points: Vec<AccessPoint>,
    /// Saved networks: name to the connection's object path. `None` when not read this time.
    known: Option<HashMap<String, String>>,
}

pub struct WifiPage {
    pub widget: gtk::ScrolledWindow,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    bus: Rc<bus::SystemRefresh>,
    status: ndl::ActionRow,
    radio: ndl::SwitchRow,
    networks: ndl::PreferencesGroup,
    rows: RefCell<Vec<ndl::ActionRow>>,
    shown: RefCell<Vec<(AccessPoint, bool, bool)>>,
    prompt: ndl::PreferencesGroup,
    password: ndl::PasswordEntryRow,
    connect: gtk::Button,
    wanted: RefCell<Option<AccessPoint>>,
    device: RefCell<Option<String>>,
    active: RefCell<Option<String>>,
    known: RefCell<HashMap<String, String>>,
    polls: Cell<u32>,
    loading: Cell<bool>,
}

async fn read(connection: &gio::DBusConnection, with_known: bool) -> Result<Reading, String> {
    let manager = bus::properties(connection, NM, NM_PATH, NM)
        .await
        .ok_or_else(|| tr("NetworkManager is not available.").to_string())?;
    let enabled = manager
        .get("WirelessEnabled")
        .and_then(|v| v.get::<bool>())
        .unwrap_or(false);

    let devices = bus::call(connection, NM, NM_PATH, NM, "GetDevices", None)
        .await
        .ok()
        .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
        .map(|(d,)| d)
        .unwrap_or_default();
    let mut device = None;
    for path in devices {
        let props = bus::properties(connection, NM, path.as_str(), DEVICE).await;
        let kind = props
            .and_then(|p| p.get("DeviceType").and_then(|v| v.get::<u32>()))
            .unwrap_or(0);
        if kind == DEVICE_TYPE_WIFI {
            device = Some(path.as_str().to_owned());
            break;
        }
    }
    let mut reading = Reading {
        enabled,
        device: device.clone(),
        active: None,
        points: Vec::new(),
        known: None,
    };
    let Some(device) = device else {
        return Ok(reading);
    };

    if enabled {
        let wireless = bus::properties(connection, NM, &device, WIRELESS).await;
        let active_path = wireless
            .as_ref()
            .and_then(|w| w.get("ActiveAccessPoint"))
            .and_then(|v| v.get::<glib::variant::ObjectPath>())
            .map(|p| p.as_str().to_owned())
            .filter(|p| p != "/");
        let paths = bus::call(
            connection,
            NM,
            &device,
            WIRELESS,
            "GetAllAccessPoints",
            None,
        )
        .await
        .ok()
        .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
        .map(|(p,)| p)
        .unwrap_or_default();
        for path in paths {
            let Some(p) = bus::properties(connection, NM, path.as_str(), ACCESS_POINT).await else {
                continue;
            };
            let flag = |name: &str| p.get(name).and_then(|v| v.get::<u32>()).unwrap_or(0);
            let point = AccessPoint {
                path: path.as_str().to_owned(),
                ssid: network::ssid_text(
                    &p.get("Ssid")
                        .and_then(|v| v.get::<Vec<u8>>())
                        .unwrap_or_default(),
                ),
                strength: p.get("Strength").and_then(|v| v.get::<u8>()).unwrap_or(0),
                secured: network::is_secured(flag("Flags"), flag("WpaFlags"), flag("RsnFlags")),
                enterprise: network::is_enterprise(flag("WpaFlags"), flag("RsnFlags")),
            };
            if active_path.as_deref() == Some(path.as_str()) {
                reading.active = Some(point.ssid.clone());
            }
            reading.points.push(point);
        }
    }
    if with_known {
        reading.known = Some(read_known(connection).await);
    }
    Ok(reading)
}

/// The saved Wi-Fi networks by name.
async fn read_known(connection: &gio::DBusConnection) -> HashMap<String, String> {
    let mut known = HashMap::new();
    let paths = bus::call(
        connection,
        NM,
        SETTINGS_PATH,
        SETTINGS,
        "ListConnections",
        None,
    )
    .await
    .ok()
    .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
    .map(|(p,)| p)
    .unwrap_or_default();
    for path in paths {
        let Ok(reply) = bus::call(
            connection,
            NM,
            path.as_str(),
            CONNECTION,
            "GetSettings",
            None,
        )
        .await
        else {
            continue;
        };
        let Some((settings,)) = reply.get::<(HashMap<String, HashMap<String, glib::Variant>>,)>()
        else {
            continue;
        };
        if let Some(ssid) = settings
            .get("802-11-wireless")
            .and_then(|w| w.get("ssid"))
            .and_then(|v| v.get::<Vec<u8>>())
        {
            known.insert(network::ssid_text(&ssid), path.as_str().to_owned());
        }
    }
    known
}

/// The settings of a new connection to `point`, with `password` when it has one.
fn new_connection(point: &AccessPoint, password: Option<&str>) -> glib::Variant {
    let section = |entries: Vec<(&str, glib::Variant)>| -> HashMap<String, glib::Variant> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect()
    };
    let mut settings: HashMap<String, HashMap<String, glib::Variant>> = HashMap::new();
    settings.insert(
        "connection".into(),
        section(vec![
            ("type", "802-11-wireless".to_variant()),
            ("id", point.ssid.to_variant()),
        ]),
    );
    settings.insert(
        "802-11-wireless".into(),
        section(vec![
            ("ssid", point.ssid.as_bytes().to_vec().to_variant()),
            ("mode", "infrastructure".to_variant()),
        ]),
    );
    if let Some(password) = password {
        settings.insert(
            "802-11-wireless-security".into(),
            section(vec![
                ("key-mgmt", "wpa-psk".to_variant()),
                ("psk", password.to_variant()),
            ]),
        );
    }
    settings.to_variant()
}

impl WifiPage {
    pub fn new() -> Rc<Self> {
        let content = column();

        let status = ndl::ActionRow::builder()
            .title(tr("Looking for Wi-Fi…"))
            .build();
        let radio = ndl::SwitchRow::builder().title(tr("Wi-Fi")).build();
        radio.set_sensitive(false);
        let top = ndl::PreferencesGroup::new();
        top.add(&radio);
        top.add(&status);
        content.append(&top);

        let prompt = ndl::PreferencesGroup::new();
        let password = ndl::PasswordEntryRow::builder()
            .title(tr("Password"))
            .build();
        let connect = gtk::Button::with_label(tr("Connect"));
        connect.add_css_class("suggested-action");
        connect.set_valign(gtk::Align::Center);
        connect.set_sensitive(false);
        let cancel = gtk::Button::with_label(tr("Cancel"));
        cancel.set_valign(gtk::Align::Center);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.append(&cancel);
        actions.append(&connect);
        password.add_suffix(&actions);
        prompt.add(&password);
        prompt.set_visible(false);
        content.append(&prompt);

        let networks = ndl::PreferencesGroup::builder()
            .title(tr("Networks"))
            .build();
        networks.set_visible(false);
        content.append(&networks);

        let page = Rc::new(Self {
            widget: scroller(&content),
            toasts: RefCell::new(None),
            bus: Rc::default(),
            status,
            radio,
            networks,
            rows: RefCell::new(Vec::new()),
            shown: RefCell::new(Vec::new()),
            prompt,
            password,
            connect,
            wanted: RefCell::new(None),
            device: RefCell::new(None),
            active: RefCell::new(None),
            known: RefCell::new(HashMap::new()),
            polls: Cell::new(0),
            loading: Cell::new(true),
        });

        {
            let page = page.clone();
            page.radio.clone().connect_active_notify(move |row| {
                if page.loading.get() {
                    return;
                }
                let on = row.is_active();
                let page = page.clone();
                glib::spawn_future_local(async move {
                    page.set_radio(on).await;
                });
            });
        }
        {
            let page2 = page.clone();
            page.password.connect_changed(move |row| {
                page2
                    .connect
                    .set_sensitive(network::password_is_valid(&row.text()));
            });
        }
        {
            let page2 = page.clone();
            page.connect.connect_clicked(move |_| {
                let page = page2.clone();
                glib::spawn_future_local(async move { page.submit_password().await });
            });
        }
        {
            let page2 = page.clone();
            cancel.connect_clicked(move |_| page2.hide_prompt());
        }
        // The networks come and go, so look again now and then, but only while on screen.
        {
            let weak = Rc::downgrade(&page);
            glib::timeout_add_seconds_local(POLL_SECONDS, move || {
                let Some(page) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                if page.widget.is_mapped() {
                    page.refresh();
                }
                glib::ControlFlow::Continue
            });
        }
        page.refresh();
        page
    }

    pub fn set_toasts(&self, toasts: &ndl::ToastOverlay) {
        *self.toasts.borrow_mut() = Some(toasts.clone());
    }

    fn toast(&self, text: &str) {
        if let Some(toasts) = &*self.toasts.borrow() {
            toasts.add_toast(ndl::Toast::new(text));
        }
    }

    fn hide_prompt(&self) {
        self.prompt.set_visible(false);
        self.password.set_text("");
        *self.wanted.borrow_mut() = None;
    }

    /// Looks at NetworkManager and shows what it found.
    pub fn refresh(self: &Rc<Self>) {
        let page = self.clone();
        self.bus.run(move |connection, guard| async move {
            let result = match &connection {
                Some(c) => {
                    let polls = page.polls.get();
                    page.polls.set(polls + 1);
                    read(c, polls % KNOWN_EVERY == 0).await
                }
                None => Err(tr("NetworkManager is not available.").to_string()),
            };
            match result {
                Ok(reading) => page.show(reading),
                Err(message) => page.show_unavailable(&message),
            }
            drop(guard);
        });
    }

    fn show_unavailable(&self, message: &str) {
        self.loading.set(true);
        self.status.set_title(message);
        self.radio.set_sensitive(false);
        self.networks.set_visible(false);
        self.loading.set(false);
    }

    fn show(self: &Rc<Self>, reading: Reading) {
        self.loading.set(true);
        *self.device.borrow_mut() = reading.device.clone();
        *self.active.borrow_mut() = reading.active.clone();
        if let Some(known) = reading.known {
            *self.known.borrow_mut() = known;
        }
        let has_device = reading.device.is_some();
        self.radio.set_sensitive(has_device);
        self.radio.set_active(reading.enabled && has_device);
        self.status
            .set_title(match (has_device, reading.enabled, &reading.active) {
                (false, _, _) => tr("This computer has no Wi-Fi adapter."),
                (true, false, _) => tr("Wi-Fi is off."),
                (true, true, None) => tr("Not connected"),
                (true, true, Some(_)) => tr("Connected"),
            });
        self.status
            .set_subtitle(reading.active.as_deref().unwrap_or(""));
        let visible = has_device && reading.enabled;
        self.networks.set_visible(visible);
        if visible {
            self.show_networks(network::visible(reading.points));
        }
        self.loading.set(false);
    }

    /// Shows the list, rebuilt only when it differs from what is on screen.
    fn show_networks(self: &Rc<Self>, points: Vec<AccessPoint>) {
        let active = self.active.borrow().clone();
        let known = self.known.borrow();
        let wanted: Vec<(AccessPoint, bool, bool)> = points
            .into_iter()
            .map(|p| {
                let is_active = active.as_deref() == Some(p.ssid.as_str());
                let is_known = known.contains_key(&p.ssid);
                (p, is_active, is_known)
            })
            .collect();
        drop(known);
        // A signal that moves a few points does not need the rows rebuilt.
        let key = |l: &[(AccessPoint, bool, bool)]| -> Vec<(String, u8, bool, bool, bool)> {
            l.iter()
                .map(|(p, a, k)| (p.ssid.clone(), p.strength / 20, p.secured, *a, *k))
                .collect()
        };
        if key(&self.shown.borrow()) == key(&wanted) {
            return;
        }
        for row in self.rows.borrow_mut().drain(..) {
            self.networks.remove(&row);
        }
        for (point, is_active, is_known) in &wanted {
            let row = ndl::ActionRow::builder().title(point.ssid.as_str()).build();
            row.add_prefix(&gtk::Image::from_icon_name(network::signal_icon(
                point.strength,
            )));
            if *is_active {
                row.set_subtitle(tr("Connected"));
            } else if *is_known {
                row.set_subtitle(tr("Saved"));
            }
            if point.secured {
                row.add_suffix(&gtk::Image::from_icon_name("changes-prevent-symbolic"));
            }
            if *is_active {
                let disconnect = gtk::Button::with_label(tr("Disconnect"));
                disconnect.set_valign(gtk::Align::Center);
                let page = self.clone();
                disconnect.connect_clicked(move |_| {
                    let page = page.clone();
                    glib::spawn_future_local(async move { page.disconnect().await });
                });
                row.add_suffix(&disconnect);
            } else {
                row.set_activatable(true);
                let (page, point) = (self.clone(), point.clone());
                row.connect_activated(move |_| {
                    let (page, point) = (page.clone(), point.clone());
                    glib::spawn_future_local(async move { page.choose(point).await });
                });
            }
            self.networks.add(&row);
            self.rows.borrow_mut().push(row);
        }
        *self.shown.borrow_mut() = wanted;
    }

    async fn set_radio(self: &Rc<Self>, on: bool) {
        let Some(connection) = self.bus.cached() else {
            return;
        };
        let parameters = (NM, "WirelessEnabled", on.to_variant()).to_variant();
        let result = bus::call(
            &connection,
            NM,
            NM_PATH,
            "org.freedesktop.DBus.Properties",
            "Set",
            Some(&parameters),
        )
        .await;
        if let Err(error) = result {
            self.toast(error.message());
        }
        self.refresh();
    }

    /// What a click on a network does: connect to a saved or open one, ask for the password of
    /// a new one, or send an enterprise one to the previous panel.
    async fn choose(self: &Rc<Self>, point: AccessPoint) {
        if self.known.borrow().contains_key(&point.ssid) {
            let path = self.known.borrow().get(&point.ssid).cloned();
            if let Some(path) = path {
                self.activate(&path, &point).await;
            }
        } else if point.enterprise {
            self.toast(tr(
                "This network needs a login. Use the previous Wi-Fi settings.",
            ));
            crate::legacy::open_panel(&self.widget, "wifi");
        } else if point.secured {
            self.password
                .set_title(&format!("{} {}", tr("Password for"), point.ssid));
            self.password.set_text("");
            *self.wanted.borrow_mut() = Some(point);
            self.prompt.set_visible(true);
            self.password.grab_focus();
        } else {
            self.add_and_activate(&point, None).await;
        }
    }

    async fn submit_password(self: &Rc<Self>) {
        let Some(point) = self.wanted.borrow().clone() else {
            return;
        };
        let password = self.password.text().to_string();
        if !network::password_is_valid(&password) {
            return;
        }
        self.hide_prompt();
        self.add_and_activate(&point, Some(&password)).await;
    }

    fn device_path(&self) -> Option<glib::variant::ObjectPath> {
        self.device.borrow().as_deref().and_then(bus::object_path)
    }

    async fn activate(self: &Rc<Self>, connection_path: &str, point: &AccessPoint) {
        let (Some(connection), Some(device), Some(saved), Some(ap)) = (
            self.bus.cached(),
            self.device_path(),
            bus::object_path(connection_path),
            bus::object_path(&point.path),
        ) else {
            return;
        };
        let parameters = (saved, device, ap).to_variant();
        if let Err(error) = bus::call(
            &connection,
            NM,
            NM_PATH,
            NM,
            "ActivateConnection",
            Some(&parameters),
        )
        .await
        {
            self.toast(error.message());
        }
        self.settle();
    }

    async fn add_and_activate(self: &Rc<Self>, point: &AccessPoint, password: Option<&str>) {
        let (Some(connection), Some(device), Some(ap)) = (
            self.bus.cached(),
            self.device_path(),
            bus::object_path(&point.path),
        ) else {
            return;
        };
        // A tuple of the three arguments: a variant inside a Rust tuple would be boxed.
        let parameters = glib::Variant::tuple_from_iter([
            new_connection(point, password),
            device.to_variant(),
            ap.to_variant(),
        ]);
        if let Err(error) = bus::call(
            &connection,
            NM,
            NM_PATH,
            NM,
            "AddAndActivateConnection",
            Some(&parameters),
        )
        .await
        {
            self.toast(error.message());
        }
        // The new connection is saved; read the saved ones again.
        self.polls.set(0);
        self.settle();
    }

    async fn disconnect(self: &Rc<Self>) {
        let (Some(connection), Some(device)) = (self.bus.cached(), self.device.borrow().clone())
        else {
            return;
        };
        if let Err(error) = bus::call(&connection, NM, &device, DEVICE, "Disconnect", None).await {
            self.toast(error.message());
        }
        self.settle();
    }

    /// Connecting takes a moment: look again soon, and once more after it has had time.
    fn settle(self: &Rc<Self>) {
        for seconds in [1, 4] {
            let page = self.clone();
            glib::timeout_add_seconds_local_once(seconds, move || page.refresh());
        }
    }
}
