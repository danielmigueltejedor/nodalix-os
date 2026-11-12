//! Bluetooth, through BlueZ's D-Bus API: switch the adapter, see the devices around and the
//! ones already paired, connect, disconnect, pair and forget. A pairing that asks for a code is
//! answered by the desktop's own agent.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ajustes_core::bluetooth::{self, Device};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{bus, column, scroller};
use crate::i18n::tr;

const BLUEZ: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";
const POLL_SECONDS: u32 = 4;

type Interfaces = HashMap<String, HashMap<String, glib::Variant>>;

struct Reading {
    /// The adapter's object path; none on a machine without Bluetooth.
    adapter: Option<String>,
    powered: bool,
    alias: String,
    devices: Vec<Device>,
}

pub struct BluetoothPage {
    pub widget: gtk::ScrolledWindow,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    bus: Rc<bus::SystemRefresh>,
    radio: ndl::SwitchRow,
    status: ndl::ActionRow,
    group: ndl::PreferencesGroup,
    rows: RefCell<Vec<ndl::ActionRow>>,
    shown: RefCell<Vec<Device>>,
    adapter: RefCell<Option<String>>,
    scanning: Cell<bool>,
    loading: Cell<bool>,
}

async fn read(connection: &gio::DBusConnection) -> Option<Reading> {
    let reply = bus::call(
        connection,
        BLUEZ,
        "/",
        "org.freedesktop.DBus.ObjectManager",
        "GetManagedObjects",
        None,
    )
    .await
    .ok()?;
    let (objects,) = reply.get::<(HashMap<glib::variant::ObjectPath, Interfaces>,)>()?;
    let mut reading = Reading {
        adapter: None,
        powered: false,
        alias: String::new(),
        devices: Vec::new(),
    };
    let mut paths: Vec<_> = objects.keys().cloned().collect();
    paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for path in paths {
        let interfaces = &objects[&path];
        if let Some(adapter) = interfaces.get(ADAPTER) {
            // The first adapter is the one this page drives.
            if reading.adapter.is_none() {
                reading.adapter = Some(path.as_str().to_owned());
                reading.powered = adapter
                    .get("Powered")
                    .and_then(|v| v.get::<bool>())
                    .unwrap_or(false);
                reading.alias = adapter
                    .get("Alias")
                    .and_then(|v| v.get::<String>())
                    .unwrap_or_default();
            }
        }
        if let Some(device) = interfaces.get(DEVICE) {
            let text = |key: &str| {
                device
                    .get(key)
                    .and_then(|v| v.get::<String>())
                    .unwrap_or_default()
            };
            let flag = |key: &str| {
                device
                    .get(key)
                    .and_then(|v| v.get::<bool>())
                    .unwrap_or(false)
            };
            let name = text("Alias");
            let paired = flag("Paired");
            // A stranger known only by its address is no use to pick from.
            if !paired && !bluetooth::has_real_name(&name, &text("Address")) {
                continue;
            }
            reading.devices.push(Device {
                path: path.as_str().to_owned(),
                name,
                paired,
                connected: flag("Connected"),
                icon: text("Icon"),
                battery: interfaces
                    .get(BATTERY)
                    .and_then(|b| b.get("Percentage"))
                    .and_then(|v| v.get::<u8>()),
            });
        }
    }
    reading.devices = bluetooth::ordered(reading.devices);
    Some(reading)
}

impl BluetoothPage {
    pub fn new() -> Rc<Self> {
        let content = column();
        let top = ndl::PreferencesGroup::new();
        let radio = ndl::SwitchRow::builder().title(tr("Bluetooth")).build();
        radio.add_prefix(&gtk::Image::from_icon_name("bluetooth-symbolic"));
        radio.set_sensitive(false);
        let status = ndl::ActionRow::builder()
            .title(tr("Looking for Bluetooth…"))
            .build();
        status.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
        top.add(&radio);
        top.add(&status);
        content.append(&top);

        let group = ndl::PreferencesGroup::builder()
            .title(tr("Devices"))
            .build();
        group.set_visible(false);
        content.append(&group);

        let page = Rc::new(Self {
            widget: scroller(&content),
            toasts: RefCell::new(None),
            bus: Rc::default(),
            radio,
            status,
            group,
            rows: RefCell::new(Vec::new()),
            shown: RefCell::new(Vec::new()),
            adapter: RefCell::new(None),
            scanning: Cell::new(false),
            loading: Cell::new(true),
        });
        {
            let page2 = page.clone();
            page.radio.connect_active_notify(move |row| {
                if page2.loading.get() {
                    return;
                }
                let (page, on) = (page2.clone(), row.is_active());
                glib::spawn_future_local(async move { page.set_power(on).await });
            });
        }
        // Devices only show up while the adapter is looking, so look while this page is on
        // screen and stop when it is not.
        {
            let page2 = page.clone();
            page.widget.connect_map(move |_| {
                page2.refresh();
            });
            let page2 = page.clone();
            page.widget.connect_unmap(move |_| {
                let page = page2.clone();
                glib::spawn_future_local(async move { page.scan(false).await });
            });
        }
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

    pub fn refresh(self: &Rc<Self>) {
        let page = self.clone();
        self.bus.run(move |connection, guard| async move {
            let reading = match &connection {
                Some(c) => read(c).await,
                None => None,
            };
            page.show(reading);
            drop(guard);
            // Look for devices while this page is showing and the adapter is on.
            let wanted = page.widget.is_mapped() && page.radio.is_active();
            page.scan(wanted).await;
        });
    }

    fn show(self: &Rc<Self>, reading: Option<Reading>) {
        self.loading.set(true);
        let Some(reading) = reading.filter(|r| r.adapter.is_some()) else {
            *self.adapter.borrow_mut() = None;
            self.radio.set_sensitive(false);
            self.radio.set_active(false);
            self.status
                .set_title(tr("This computer has no Bluetooth adapter."));
            self.status.set_subtitle("");
            self.group.set_visible(false);
            self.loading.set(false);
            return;
        };
        *self.adapter.borrow_mut() = reading.adapter.clone();
        self.radio.set_sensitive(true);
        self.radio.set_active(reading.powered);
        if reading.powered {
            self.status.set_title(tr("Visible as"));
            self.status.set_subtitle(&reading.alias);
        } else {
            self.status.set_title(tr("Bluetooth is off."));
            self.status.set_subtitle("");
        }
        self.group
            .set_visible(reading.powered && !reading.devices.is_empty());
        if reading.powered && *self.shown.borrow() != reading.devices {
            self.rebuild(&reading.devices);
            *self.shown.borrow_mut() = reading.devices;
        }
        self.loading.set(false);
    }

    fn rebuild(self: &Rc<Self>, devices: &[Device]) {
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        for device in devices {
            let row = ndl::ActionRow::builder()
                .title(device.name.as_str())
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(&bluetooth::icon_name(
                &device.icon,
            )));
            let state = if device.connected {
                tr("Connected")
            } else if device.paired {
                tr("Paired")
            } else {
                ""
            };
            match device.battery {
                Some(percent) if device.connected => {
                    row.set_subtitle(&format!("{state} · {percent} %"));
                    row.add_suffix(&gtk::Image::from_icon_name(&bluetooth::battery_icon(
                        percent,
                    )));
                }
                _ => row.set_subtitle(state),
            }
            let button = |label: &str| {
                let b = gtk::Button::with_label(label);
                b.set_valign(gtk::Align::Center);
                b
            };
            let main = if device.connected {
                button(tr("Disconnect"))
            } else if device.paired {
                button(tr("Connect"))
            } else {
                button(tr("Pair"))
            };
            {
                let (page, device) = (self.clone(), device.clone());
                main.connect_clicked(move |b| {
                    b.set_sensitive(false);
                    let (page, device, b) = (page.clone(), device.clone(), b.clone());
                    glib::spawn_future_local(async move {
                        page.act(&device).await;
                        b.set_sensitive(true);
                    });
                });
            }
            if device.paired {
                let forget = button(tr("Forget"));
                let (page, path) = (self.clone(), device.path.clone());
                forget.connect_clicked(move |_| {
                    let (page, path) = (page.clone(), path.clone());
                    glib::spawn_future_local(async move { page.forget(&path).await });
                });
                row.add_suffix(&forget);
            }
            row.add_suffix(&main);
            self.group.add(&row);
            self.rows.borrow_mut().push(row);
        }
    }

    async fn set_power(self: &Rc<Self>, on: bool) {
        let (Some(connection), Some(adapter)) = (self.bus.cached(), self.adapter.borrow().clone())
        else {
            return;
        };
        let parameters = (ADAPTER, "Powered", on.to_variant()).to_variant();
        let result = bus::call(
            &connection,
            BLUEZ,
            &adapter,
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

    /// Starts or stops the adapter looking for devices; asking for what is already so is fine.
    async fn scan(self: &Rc<Self>, on: bool) {
        if self.scanning.replace(on) == on {
            return;
        }
        let (Some(connection), Some(adapter)) = (self.bus.cached(), self.adapter.borrow().clone())
        else {
            self.scanning.set(false);
            return;
        };
        let method = if on {
            "StartDiscovery"
        } else {
            "StopDiscovery"
        };
        // "Already in progress" and "not ready" are not worth a message.
        let _ = bus::call(&connection, BLUEZ, &adapter, ADAPTER, method, None).await;
    }

    /// Connects or disconnects a paired device; pairs one that is not, and connects it.
    async fn act(self: &Rc<Self>, device: &Device) {
        let Some(connection) = self.bus.cached() else {
            return;
        };
        let steps: &[&str] = if device.connected {
            &["Disconnect"]
        } else if device.paired {
            &["Connect"]
        } else {
            &["Pair", "Connect"]
        };
        for method in steps {
            if let Err(error) =
                bus::call_slow(&connection, BLUEZ, &device.path, DEVICE, method, None).await
            {
                self.toast(&format!("{}: {}", device.name, error.message()));
                break;
            }
        }
        self.refresh();
    }

    async fn forget(self: &Rc<Self>, path: &str) {
        let (Some(connection), Some(adapter), Some(device)) = (
            self.bus.cached(),
            self.adapter.borrow().clone(),
            bus::object_path(path),
        ) else {
            return;
        };
        let parameters = glib::Variant::tuple_from_iter([device.to_variant()]);
        if let Err(error) = bus::call(
            &connection,
            BLUEZ,
            &adapter,
            ADAPTER,
            "RemoveDevice",
            Some(&parameters),
        )
        .await
        {
            self.toast(error.message());
        }
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_text_is_translated() {
        for text in [
            "Looking for Bluetooth…",
            "This computer has no Bluetooth adapter.",
            "Visible as",
            "Bluetooth is off.",
            "Paired",
            "Pair",
            "Forget",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
    }
}
