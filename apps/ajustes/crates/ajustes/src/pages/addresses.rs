//! IPv4 addresses and name servers of the saved wired and Wi-Fi connections, read from
//! NetworkManager and written with `nmcli`, which knows every setting it must keep in step.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ajustes_core::addresses::{self, Form, Method, Problem};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::bus;
use super::cmd::capture;
use super::nm::{CONNECTION, NM, SETTINGS, SETTINGS_PATH};
use crate::i18n::tr;

/// A saved connection the form can edit.
#[derive(Clone)]
struct Profile {
    path: String,
    uuid: String,
    name: String,
}

pub struct AddressGroup {
    pub group: ndl::PreferencesGroup,
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    profiles: RefCell<Vec<Profile>>,
    connection: ndl::ComboRow,
    method: ndl::ComboRow,
    address: ndl::EntryRow,
    prefix: ndl::EntryRow,
    gateway: ndl::EntryRow,
    auto_dns: ndl::SwitchRow,
    dns: ndl::EntryRow,
    apply: gtk::Button,
    /// Set while the form is being filled in, so that filling it is not an edit.
    loading: Cell<bool>,
}

fn problem_text(problem: Problem) -> &'static str {
    match problem {
        Problem::BadAddress => "That is not an IPv4 address",
        Problem::BadPrefix => {
            "The mask must be a length such as 24 or a mask such as 255.255.255.0"
        }
        Problem::BadGateway => "The gateway is not an IPv4 address",
        Problem::BadDns => "The name servers must be addresses, separated by spaces or commas",
    }
}

type Settings = HashMap<String, HashMap<String, glib::Variant>>;

/// The form's values out of a connection's settings.
fn form_of(settings: &Settings) -> Form {
    let empty = HashMap::new();
    let ipv4 = settings.get("ipv4").unwrap_or(&empty);
    let method = ipv4
        .get("method")
        .and_then(|v| v.get::<String>())
        .map_or(Method::Auto, |m| Method::from_nm(&m));
    let first = ipv4
        .get("address-data")
        .and_then(|v| v.get::<Vec<HashMap<String, glib::Variant>>>())
        .and_then(|list| list.into_iter().next());
    let address = first
        .as_ref()
        .and_then(|a| a.get("address").and_then(|v| v.get::<String>()))
        .unwrap_or_default();
    let prefix = first
        .as_ref()
        .and_then(|a| a.get("prefix").and_then(|v| v.get::<u32>()))
        .map(|p| p.to_string())
        .unwrap_or_default();
    let gateway = ipv4
        .get("gateway")
        .and_then(|v| v.get::<String>())
        .unwrap_or_default();
    // `dns-data` holds the servers as text; older NetworkManagers only have `dns`, with each
    // address as a number in network byte order.
    let dns: Vec<String> = ipv4
        .get("dns-data")
        .and_then(|v| v.get::<Vec<String>>())
        .or_else(|| {
            ipv4.get("dns")
                .and_then(|v| v.get::<Vec<u32>>())
                .map(|list| {
                    list.into_iter()
                        .map(|n| std::net::Ipv4Addr::from(n.to_ne_bytes()).to_string())
                        .collect()
                })
        })
        .unwrap_or_default();
    let ignore = ipv4
        .get("ignore-auto-dns")
        .and_then(|v| v.get::<bool>())
        .unwrap_or(false);
    Form {
        method,
        address,
        prefix,
        gateway,
        auto_dns: !ignore,
        dns: dns.join(" "),
    }
}

impl AddressGroup {
    pub fn new() -> Rc<Self> {
        let group = ndl::PreferencesGroup::builder()
            .title(tr("Addresses and DNS"))
            .description(tr("The IP address and name servers of a saved connection"))
            .build();
        let connection = ndl::ComboRow::builder().title(tr("Connection")).build();
        connection.add_prefix(&gtk::Image::from_icon_name("network-wired-symbolic"));
        let titles: Vec<&str> = Method::ALL.iter().map(|m| tr(m.title())).collect();
        let method = ndl::ComboRow::builder()
            .title(tr("IPv4"))
            .model(&gtk::StringList::new(&titles))
            .build();
        method.add_prefix(&gtk::Image::from_icon_name("network-workgroup-symbolic"));
        let address = ndl::EntryRow::builder().title(tr("Address")).build();
        let prefix = ndl::EntryRow::builder()
            .title(tr("Mask or prefix length"))
            .build();
        let gateway = ndl::EntryRow::builder().title(tr("Gateway")).build();
        let auto_dns = ndl::SwitchRow::builder()
            .title(tr("Also use the name servers of the network"))
            .build();
        auto_dns.set_active(true);
        let dns = ndl::EntryRow::builder().title(tr("Name servers")).build();
        let apply = gtk::Button::with_label(tr("Apply"));
        apply.add_css_class("suggested-action");
        apply.set_valign(gtk::Align::Center);
        let footer = ndl::ActionRow::builder()
            .title(tr("The connection restarts to apply the change"))
            .build();
        footer.add_suffix(&apply);
        for row in [
            connection.clone().upcast::<gtk::Widget>(),
            method.clone().upcast(),
            address.clone().upcast(),
            prefix.clone().upcast(),
            gateway.clone().upcast(),
            auto_dns.clone().upcast(),
            dns.clone().upcast(),
            footer.upcast(),
        ] {
            group.add(&row);
        }
        group.set_visible(false);
        let this = Rc::new(Self {
            group,
            toasts: RefCell::new(None),
            profiles: RefCell::new(Vec::new()),
            connection,
            method,
            address,
            prefix,
            gateway,
            auto_dns,
            dns,
            apply,
            loading: Cell::new(false),
        });
        this.show_rows();
        {
            let this2 = this.clone();
            this.method
                .connect_selected_notify(move |_| this2.show_rows());
            let this2 = this.clone();
            this.connection
                .connect_selected_notify(move |_| this2.load_selected());
            let this2 = this.clone();
            this.apply.connect_clicked(move |button| {
                this2.apply_changes(button);
            });
        }
        let this2 = this.clone();
        glib::spawn_future_local(async move {
            this2.reload_profiles().await;
        });
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

    fn method_now(&self) -> Method {
        Method::ALL
            .get(self.method.selected() as usize)
            .copied()
            .unwrap_or(Method::Auto)
    }

    /// Only the rows the chosen method uses.
    fn show_rows(&self) {
        let method = self.method_now();
        let manual = method == Method::Manual;
        self.address.set_visible(manual);
        self.prefix.set_visible(manual);
        self.gateway.set_visible(manual);
        let dns_used = matches!(method, Method::Auto | Method::Manual);
        self.auto_dns.set_visible(method == Method::Auto);
        self.dns.set_visible(dns_used);
    }

    fn selected(&self) -> Option<Profile> {
        self.profiles
            .borrow()
            .get(self.connection.selected() as usize)
            .cloned()
    }

    /// The saved wired and Wi-Fi connections.
    async fn reload_profiles(self: &Rc<Self>) {
        let Some(system) = bus::system().await else {
            return;
        };
        let Some(paths) = bus::call(
            &system,
            NM,
            SETTINGS_PATH,
            SETTINGS,
            "ListConnections",
            None,
        )
        .await
        .ok()
        .and_then(|r| r.get::<(Vec<glib::variant::ObjectPath>,)>())
        .map(|(p,)| p) else {
            return;
        };
        let mut found = Vec::new();
        for path in paths {
            let Some(settings) = settings_of(&system, path.as_str()).await else {
                continue;
            };
            let Some(header) = settings.get("connection") else {
                continue;
            };
            let text = |key: &str| {
                header
                    .get(key)
                    .and_then(|v| v.get::<String>())
                    .unwrap_or_default()
            };
            if !matches!(text("type").as_str(), "802-3-ethernet" | "802-11-wireless") {
                continue;
            }
            found.push(Profile {
                path: path.as_str().to_owned(),
                uuid: text("uuid"),
                name: text("id"),
            });
        }
        found.sort_by_key(|p| p.name.to_lowercase());
        self.group.set_visible(!found.is_empty());
        let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
        self.loading.set(true);
        *self.profiles.borrow_mut() = found.clone();
        self.connection
            .set_model(Some(&gtk::StringList::new(&names)));
        self.connection.set_selected(0);
        self.loading.set(false);
        self.load_selected();
    }

    fn load_selected(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        let Some(profile) = self.selected() else {
            return;
        };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some(system) = bus::system().await else {
                return;
            };
            let Some(settings) = settings_of(&system, &profile.path).await else {
                return;
            };
            let form = form_of(&settings);
            this.loading.set(true);
            this.method.set_selected(
                Method::ALL
                    .iter()
                    .position(|m| *m == form.method)
                    .unwrap_or(0) as u32,
            );
            this.address.set_text(&form.address);
            this.prefix.set_text(&form.prefix);
            this.gateway.set_text(&form.gateway);
            this.auto_dns.set_active(form.auto_dns);
            this.dns.set_text(&form.dns);
            this.loading.set(false);
            this.show_rows();
        });
    }

    fn apply_changes(self: &Rc<Self>, button: &gtk::Button) {
        let Some(profile) = self.selected() else {
            return;
        };
        let form = Form {
            method: self.method_now(),
            address: self.address.text().to_string(),
            prefix: self.prefix.text().to_string(),
            gateway: self.gateway.text().to_string(),
            auto_dns: self.auto_dns.is_active(),
            dns: self.dns.text().to_string(),
        };
        if let Err(problem) = addresses::check(&form) {
            self.toast(tr(problem_text(problem)));
            return;
        }
        button.set_sensitive(false);
        let (this, button) = (self.clone(), button.clone());
        glib::spawn_future_local(async move {
            let args = addresses::modify_args(&profile.uuid, &form);
            let argv: Vec<&str> = args.iter().map(String::as_str).collect();
            let result = capture(&argv, None).await;
            if result.ok {
                // Applying it again is what makes a running connection take the change; a
                // connection that is not running stays off.
                let up = capture(&["nmcli", "connection", "up", "uuid", &profile.uuid], None).await;
                if !up.ok {
                    this.toast(tr(
                        "Saved. It will apply the next time the connection starts",
                    ));
                } else {
                    this.toast(tr("Applied"));
                }
            } else {
                this.toast(result.err.trim());
            }
            button.set_sensitive(true);
        });
    }
}

async fn settings_of(connection: &gio::DBusConnection, path: &str) -> Option<Settings> {
    bus::call(connection, NM, path, CONNECTION, "GetSettings", None)
        .await
        .ok()?
        .get::<(Settings,)>()
        .map(|(s,)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_is_translated() {
        for text in [
            "Addresses and DNS",
            "The IP address and name servers of a saved connection",
            "Connection",
            "IPv4",
            "Address",
            "Mask or prefix length",
            "Gateway",
            "Also use the name servers of the network",
            "Name servers",
            "The connection restarts to apply the change",
            "Saved. It will apply the next time the connection starts",
            "Applied",
            "Automatic (DHCP)",
            "Manual",
            "Link-local only",
            "Disabled",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
        for problem in [
            Problem::BadAddress,
            Problem::BadPrefix,
            Problem::BadGateway,
            Problem::BadDns,
        ] {
            assert!(crate::i18n::has_translation(problem_text(problem)));
        }
    }

    #[test]
    fn the_form_comes_from_the_settings() {
        let mut ipv4: HashMap<String, glib::Variant> = HashMap::new();
        ipv4.insert("method".into(), "manual".to_variant());
        ipv4.insert("gateway".into(), "10.0.0.1".to_variant());
        ipv4.insert("ignore-auto-dns".into(), true.to_variant());
        ipv4.insert(
            "dns-data".into(),
            vec!["1.1.1.1".to_owned(), "9.9.9.9".to_owned()].to_variant(),
        );
        let mut address: HashMap<String, glib::Variant> = HashMap::new();
        address.insert("address".into(), "10.0.0.5".to_variant());
        address.insert("prefix".into(), 24u32.to_variant());
        ipv4.insert("address-data".into(), vec![address].to_variant());
        let mut settings = Settings::new();
        settings.insert("ipv4".into(), ipv4);
        let form = form_of(&settings);
        assert_eq!(form.method, Method::Manual);
        assert_eq!(form.address, "10.0.0.5");
        assert_eq!(form.prefix, "24");
        assert_eq!(form.gateway, "10.0.0.1");
        assert!(!form.auto_dns);
        assert_eq!(form.dns, "1.1.1.1 9.9.9.9");
        // No ipv4 section is an automatic connection.
        assert_eq!(form_of(&Settings::new()).method, Method::Auto);
    }
}
