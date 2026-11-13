//! Network: whether there is internet, and the wired adapters with their address and a switch to
//! connect or disconnect each, the VPN connections and the proxy. Addresses and DNS stay in the
//! previous panel.

use std::cell::RefCell;
use std::rc::Rc;

use ajustes_core::network::{self as core, DeviceState};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::nm::{DEVICE, IP4_CONFIG, NM, NM_PATH};
use super::{addresses, bus, column, proxy, scroller, vpn};
use crate::i18n::tr;

const POLL_SECONDS: u32 = 5;

#[derive(Clone, PartialEq, Eq)]
struct Adapter {
    path: String,
    name: String,
    state: DeviceState,
    address: Option<String>,
}

struct Reading {
    /// `NMConnectivityState`: 4 is full internet, 3 limited, 2 behind a portal, 1 none.
    connectivity: u32,
    adapters: Vec<Adapter>,
}

pub struct NetworkPage {
    pub widget: gtk::ScrolledWindow,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    bus: Rc<bus::SystemRefresh>,
    internet: ndl::ActionRow,
    internet_icon: gtk::Image,
    vpn: Rc<vpn::VpnGroup>,
    addresses: Rc<addresses::AddressGroup>,
    wired: ndl::PreferencesGroup,
    rows: RefCell<Vec<ndl::ActionRow>>,
    shown: RefCell<Vec<Adapter>>,
}

/// The first IPv4 address in `AddressData`, a list of dictionaries with an `address` text.
fn first_address(data: &glib::Variant) -> Option<String> {
    let list = data.get::<Vec<std::collections::HashMap<String, glib::Variant>>>()?;
    list.first()?.get("address")?.get::<String>()
}

async fn read(connection: &gio::DBusConnection) -> Option<Reading> {
    let manager = bus::properties(connection, NM, NM_PATH, NM).await?;
    let connectivity = manager
        .get("Connectivity")
        .and_then(|v| v.get::<u32>())
        .unwrap_or(0);
    let paths = bus::call(connection, NM, NM_PATH, NM, "GetDevices", None)
        .await
        .ok()
        .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
        .map(|(p,)| p)
        .unwrap_or_default();
    let mut adapters = Vec::new();
    for path in paths {
        let Some(props) = bus::properties(connection, NM, path.as_str(), DEVICE).await else {
            continue;
        };
        if props.get("DeviceType").and_then(|v| v.get::<u32>()) != Some(core::DEVICE_TYPE_ETHERNET)
        {
            continue;
        }
        let state =
            core::device_state(props.get("State").and_then(|v| v.get::<u32>()).unwrap_or(0));
        let mut address = None;
        if let Some(config) = props
            .get("Ip4Config")
            .and_then(|v| v.get::<glib::variant::ObjectPath>())
            .filter(|p| p.as_str() != "/")
        {
            address = bus::properties(connection, NM, config.as_str(), IP4_CONFIG)
                .await
                .and_then(|c| c.get("AddressData").and_then(first_address));
        }
        adapters.push(Adapter {
            path: path.as_str().to_owned(),
            name: props
                .get("Interface")
                .and_then(|v| v.get::<String>())
                .unwrap_or_default(),
            state,
            address,
        });
    }
    Some(Reading {
        connectivity,
        adapters,
    })
}

/// Icons for a connectivity state, each list tried in order: a theme may lack some names.
fn connectivity_icons(state: u32) -> &'static [&'static str] {
    match state {
        4 => &[
            "network-transmit-receive-symbolic",
            "network-wired-symbolic",
        ],
        3 | 2 => &["network-error-symbolic", "network-wired-symbolic"],
        _ => &[
            "network-offline-symbolic",
            "network-wired-disconnected-symbolic",
        ],
    }
}

fn connectivity_text(state: u32) -> &'static str {
    match state {
        4 => tr("Connected to the internet"),
        3 => tr("Limited connection"),
        2 => tr("Sign in to the network"),
        1 => tr("No internet"),
        _ => tr("Unknown"),
    }
}

fn state_text(state: DeviceState) -> &'static str {
    match state {
        DeviceState::Connected => tr("Connected"),
        DeviceState::Connecting => tr("Connecting…"),
        DeviceState::Disconnected => tr("Not connected"),
        DeviceState::Failed => tr("Could not connect"),
        DeviceState::Unavailable => tr("Cable unplugged"),
    }
}

impl NetworkPage {
    pub fn new() -> Rc<Self> {
        let content = column();
        let top = ndl::PreferencesGroup::new();
        let internet = ndl::ActionRow::builder()
            .title(tr("Looking for the network…"))
            .build();
        let internet_icon = gtk::Image::from_icon_name("network-offline-symbolic");
        internet.add_prefix(&internet_icon);
        top.add(&internet);
        content.append(&top);

        let wired = ndl::PreferencesGroup::builder().title(tr("Wired")).build();
        wired.set_visible(false);
        content.append(&wired);

        let vpn = vpn::VpnGroup::new();
        content.append(&vpn.group);
        for group in proxy::groups() {
            content.append(&group);
        }
        let addresses = addresses::AddressGroup::new();
        content.append(&addresses.group);

        let page = Rc::new(Self {
            widget: scroller(&content),
            toasts: RefCell::new(None),
            bus: Rc::default(),
            internet,
            internet_icon,
            vpn,
            addresses,
            wired,
            rows: RefCell::new(Vec::new()),
            shown: RefCell::new(Vec::new()),
        });
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
        self.vpn.set_toasts(toasts);
        self.addresses.set_toasts(toasts);
    }

    fn toast(&self, text: &str) {
        if let Some(toasts) = &*self.toasts.borrow() {
            toasts.add_toast(ndl::Toast::new(text));
        }
    }

    pub fn refresh(self: &Rc<Self>) {
        self.vpn.refresh();
        let page = self.clone();
        self.bus.run(move |connection, guard| async move {
            let reading = match &connection {
                Some(c) => read(c).await,
                None => None,
            };
            match reading {
                Some(reading) => page.show(reading),
                None => {
                    page.internet
                        .set_title(tr("NetworkManager is not available."));
                    page.wired.set_visible(false);
                }
            }
            drop(guard);
        });
    }

    fn show(self: &Rc<Self>, reading: Reading) {
        self.internet
            .set_title(connectivity_text(reading.connectivity));
        self.internet_icon
            .set_from_gicon(&gio::ThemedIcon::from_names(connectivity_icons(
                reading.connectivity,
            )));
        self.wired.set_visible(!reading.adapters.is_empty());
        if *self.shown.borrow() == reading.adapters {
            return;
        }
        for row in self.rows.borrow_mut().drain(..) {
            self.wired.remove(&row);
        }
        for adapter in &reading.adapters {
            let row = ndl::ActionRow::builder()
                .title(adapter.name.as_str())
                .build();
            let detail = match &adapter.address {
                Some(address) if adapter.state == DeviceState::Connected => {
                    format!("{} · {address}", state_text(adapter.state))
                }
                _ => state_text(adapter.state).to_string(),
            };
            row.set_subtitle(&detail);
            row.add_prefix(&gtk::Image::from_icon_name("network-wired-symbolic"));
            if matches!(
                adapter.state,
                DeviceState::Connected | DeviceState::Disconnected | DeviceState::Failed
            ) {
                let connected = adapter.state == DeviceState::Connected;
                let button = gtk::Button::with_label(if connected {
                    tr("Disconnect")
                } else {
                    tr("Connect")
                });
                button.set_valign(gtk::Align::Center);
                let (page, path) = (self.clone(), adapter.path.clone());
                button.connect_clicked(move |_| {
                    let (page, path) = (page.clone(), path.clone());
                    glib::spawn_future_local(async move { page.switch(&path, connected).await });
                });
                row.add_suffix(&button);
            }
            self.wired.add(&row);
            self.rows.borrow_mut().push(row);
        }
        *self.shown.borrow_mut() = reading.adapters;
    }

    /// Disconnects the adapter, or connects it with whichever saved connection suits it
    /// (NetworkManager picks one when it is given "/").
    async fn switch(self: &Rc<Self>, device: &str, connected: bool) {
        let Some(connection) = self.bus.cached() else {
            return;
        };
        let result = if connected {
            bus::call(&connection, NM, device, DEVICE, "Disconnect", None).await
        } else {
            let (Some(device), Some(root)) = (bus::object_path(device), bus::object_path("/"))
            else {
                return;
            };
            let parameters = glib::Variant::tuple_from_iter([
                root.to_variant(),
                device.to_variant(),
                root.to_variant(),
            ]);
            bus::call(
                &connection,
                NM,
                NM_PATH,
                NM,
                "ActivateConnection",
                Some(&parameters),
            )
            .await
        };
        if let Err(error) = result {
            self.toast(error.message());
        }
        for seconds in [1, 4] {
            let page = self.clone();
            glib::timeout_add_seconds_local_once(seconds, move || page.refresh());
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn every_text_is_translated() {
        for text in [
            "Connected to the internet",
            "Limited connection",
            "Sign in to the network",
            "No internet",
            "Connecting…",
            "Could not connect",
            "Cable unplugged",
            "Wired",
            "Addresses and DNS",
            "Set the IP address and name servers of a connection",
            "Looking for the network…",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
    }
}
