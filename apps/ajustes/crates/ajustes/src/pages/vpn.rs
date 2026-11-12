//! VPN connections, through NetworkManager: the ones that are saved (WireGuard and the plug-in
//! kinds such as OpenVPN), to connect, disconnect and remove, and importing a configuration file.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::bus;
use super::cmd::capture;
use super::nm::{CONNECTION, NM, NM_PATH, SETTINGS, SETTINGS_PATH};
use crate::i18n::tr;

const ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";

#[derive(Clone, PartialEq, Eq)]
struct Vpn {
    /// The saved connection's object path.
    path: String,
    name: String,
    /// The active connection's path while it is connected or connecting.
    active: Option<String>,
}

/// The saved VPN connections, each with its active connection if there is one.
async fn read(connection: &gio::DBusConnection) -> Option<Vec<Vpn>> {
    let paths = bus::call(
        connection,
        NM,
        SETTINGS_PATH,
        SETTINGS,
        "ListConnections",
        None,
    )
    .await
    .ok()?
    .get::<(Vec<glib::variant::ObjectPath>,)>()?
    .0;
    let mut active: HashMap<String, String> = HashMap::new();
    if let Some(all) = bus::properties(connection, NM, NM_PATH, NM).await {
        let actives = all
            .get("ActiveConnections")
            .and_then(|v| v.get::<Vec<glib::variant::ObjectPath>>())
            .unwrap_or_default();
        for path in actives {
            if let Some(props) = bus::properties(connection, NM, path.as_str(), ACTIVE).await {
                if let Some(saved) = props
                    .get("Connection")
                    .and_then(|v| v.get::<glib::variant::ObjectPath>())
                {
                    active.insert(saved.as_str().to_owned(), path.as_str().to_owned());
                }
            }
        }
    }
    let mut out = Vec::new();
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
        let Some(header) = settings.get("connection") else {
            continue;
        };
        let kind = header
            .get("type")
            .and_then(|v| v.get::<String>())
            .unwrap_or_default();
        if kind != "vpn" && kind != "wireguard" {
            continue;
        }
        out.push(Vpn {
            path: path.as_str().to_owned(),
            name: header
                .get("id")
                .and_then(|v| v.get::<String>())
                .unwrap_or_default(),
            active: active.get(path.as_str()).cloned(),
        });
    }
    out.sort_by_key(|vpn| vpn.name.to_lowercase());
    Some(out)
}

/// The `nmcli` connection type for a configuration file, by its extension.
fn import_type(file: &str) -> Option<&'static str> {
    match file.rsplit('.').next()?.to_lowercase().as_str() {
        "conf" => Some("wireguard"),
        "ovpn" => Some("openvpn"),
        _ => None,
    }
}

pub struct VpnGroup {
    pub group: ndl::PreferencesGroup,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    rows: RefCell<Vec<ndl::ActionRow>>,
    shown: RefCell<Vec<Vpn>>,
    busy: Cell<bool>,
}

impl VpnGroup {
    pub fn new() -> Rc<Self> {
        let group = ndl::PreferencesGroup::builder().title(tr("VPN")).build();
        let import = ndl::ActionRow::builder()
            .title(tr("Import a VPN configuration"))
            .subtitle(tr("A WireGuard (.conf) or OpenVPN (.ovpn) file"))
            .build();
        import.add_prefix(&gtk::Image::from_icon_name("document-open-symbolic"));
        let choose = gtk::Button::with_label(tr("Choose a file"));
        choose.set_valign(gtk::Align::Center);
        import.add_suffix(&choose);
        group.add(&import);
        let this = Rc::new(Self {
            group,
            toasts: RefCell::new(None),
            rows: RefCell::new(Vec::new()),
            shown: RefCell::new(Vec::new()),
            busy: Cell::new(false),
        });
        {
            let this = this.clone();
            choose.connect_clicked(move |button| {
                let window = button.root().and_downcast::<gtk::Window>();
                let this = this.clone();
                glib::spawn_future_local(async move { this.import(window).await });
            });
        }
        this
    }

    pub fn set_toasts(&self, toasts: &ndl::ToastOverlay) {
        *self.toasts.borrow_mut() = Some(toasts.clone());
    }

    fn toast(&self, text: &str) {
        if let Some(toasts) = &*self.toasts.borrow() {
            toasts.add_toast(ndl::Toast::new(text));
        }
    }

    async fn import(self: &Rc<Self>, window: Option<gtk::Window>) {
        let dialog = gtk::FileDialog::builder()
            .title(tr("Import a VPN configuration"))
            .build();
        let Ok(file) = dialog.open_future(window.as_ref()).await else {
            return;
        };
        let Some(path) = file.path() else {
            return;
        };
        let path = path.to_string_lossy().into_owned();
        let Some(kind) = import_type(&path) else {
            self.toast(tr("Choose a .conf (WireGuard) or .ovpn (OpenVPN) file"));
            return;
        };
        let output = capture(
            &["nmcli", "connection", "import", "type", kind, "file", &path],
            None,
        )
        .await;
        self.toast(tr(if output.ok {
            "VPN added"
        } else {
            "The configuration could not be imported"
        }));
        self.refresh();
    }

    pub fn refresh(self: &Rc<Self>) {
        if self.busy.replace(true) {
            return;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let found = match bus::system().await {
                Some(connection) => read(&connection).await,
                None => None,
            };
            this.busy.set(false);
            // Without NetworkManager there is nothing to list, and the import would go nowhere.
            this.group.set_visible(found.is_some());
            if let Some(list) = found {
                this.show(list);
            }
        });
    }

    fn show(self: &Rc<Self>, list: Vec<Vpn>) {
        if *self.shown.borrow() == list {
            return;
        }
        for old in self.rows.borrow_mut().drain(..) {
            self.group.remove(&old);
        }
        for vpn in &list {
            let row = ndl::ActionRow::builder().title(vpn.name.as_str()).build();
            row.set_subtitle(tr(if vpn.active.is_some() {
                "Connected"
            } else {
                "Not connected"
            }));
            row.add_prefix(&gtk::Image::from_gicon(&gio::ThemedIcon::from_names(&[
                "network-vpn-symbolic",
                "channel-secure-symbolic",
            ])));
            let on = vpn.active.is_some();
            let toggle = gtk::Button::with_label(tr(if on { "Disconnect" } else { "Connect" }));
            toggle.set_valign(gtk::Align::Center);
            let remove = gtk::Button::from_icon_name("user-trash-symbolic");
            remove.set_valign(gtk::Align::Center);
            remove.set_tooltip_text(Some(tr("Remove")));
            remove.add_css_class("flat");
            row.add_suffix(&toggle);
            row.add_suffix(&remove);
            {
                let (this, vpn) = (self.clone(), vpn.clone());
                toggle.connect_clicked(move |button| {
                    button.set_sensitive(false);
                    let (this, vpn) = (this.clone(), vpn.clone());
                    glib::spawn_future_local(async move { this.switch(&vpn).await });
                });
            }
            {
                let (this, vpn) = (self.clone(), vpn.clone());
                remove.connect_clicked(move |button| {
                    let (this, vpn) = (this.clone(), vpn.clone());
                    let window = button.root().and_downcast::<gtk::Window>();
                    glib::spawn_future_local(async move {
                        let dialog = gtk::AlertDialog::builder()
                            .message(tr("Remove this VPN?"))
                            .detail(vpn.name.as_str())
                            .buttons([tr("Cancel"), tr("Remove")])
                            .cancel_button(0)
                            .default_button(0)
                            .modal(true)
                            .build();
                        if dialog.choose_future(window.as_ref()).await == Ok(1) {
                            this.remove(&vpn).await;
                        }
                    });
                });
            }
            self.group.add(&row);
            self.rows.borrow_mut().push(row);
        }
        *self.shown.borrow_mut() = list;
    }

    async fn switch(self: &Rc<Self>, vpn: &Vpn) {
        let Some(connection) = bus::system().await else {
            return;
        };
        let result = match &vpn.active {
            Some(active) => match bus::object_path(active) {
                Some(active) => {
                    bus::call(
                        &connection,
                        NM,
                        NM_PATH,
                        NM,
                        "DeactivateConnection",
                        Some(&(active,).to_variant()),
                    )
                    .await
                }
                None => return,
            },
            None => {
                let (Some(saved), Some(root)) =
                    (bus::object_path(&vpn.path), bus::object_path("/"))
                else {
                    return;
                };
                let parameters = glib::Variant::tuple_from_iter([
                    saved.to_variant(),
                    root.to_variant(),
                    root.to_variant(),
                ]);
                bus::call_slow(
                    &connection,
                    NM,
                    NM_PATH,
                    NM,
                    "ActivateConnection",
                    Some(&parameters),
                )
                .await
            }
        };
        if let Err(error) = result {
            self.toast(error.message());
        }
        self.refresh();
    }

    async fn remove(self: &Rc<Self>, vpn: &Vpn) {
        let Some(connection) = bus::system().await else {
            return;
        };
        if let Err(error) = bus::call(&connection, NM, &vpn.path, CONNECTION, "Delete", None).await
        {
            self.toast(error.message());
        }
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_imported_as_what_its_extension_says() {
        assert_eq!(import_type("/home/u/wg0.conf"), Some("wireguard"));
        assert_eq!(import_type("work.OVPN"), Some("openvpn"));
        assert_eq!(import_type("notes.txt"), None);
        assert_eq!(import_type("noextension"), None);
    }

    #[test]
    fn every_text_is_translated() {
        for text in [
            "VPN",
            "Import a VPN configuration",
            "A WireGuard (.conf) or OpenVPN (.ovpn) file",
            "Choose a file",
            "Choose a .conf (WireGuard) or .ovpn (OpenVPN) file",
            "VPN added",
            "The configuration could not be imported",
            "Remove this VPN?",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
    }
}
