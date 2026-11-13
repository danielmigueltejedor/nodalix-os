//! Sharing: the name of this device, as the network sees it. File sharing between devices is
//! the LocalSend page.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{bus, column, scroller};
use crate::i18n::tr;

const HOSTNAME: &str = "org.freedesktop.hostname1";
const HOSTNAME_PATH: &str = "/org/freedesktop/hostname1";

/// The network name that goes with a device name typed as text: lowercase letters, digits and
/// hyphens, no longer than a host name may be.
pub fn static_hostname(pretty: &str) -> String {
    let mut name = String::new();
    for c in pretty.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            name.push(c);
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
    }
    let name = name.trim_end_matches('-');
    name.chars()
        .take(63)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned()
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("This device"))
        .build();
    let name = ndl::EntryRow::builder().title(tr("Device name")).build();
    name.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
    let apply = gtk::Button::with_label(tr("Apply"));
    apply.set_valign(gtk::Align::Center);
    apply.set_sensitive(false);
    name.add_suffix(&apply);
    name.set_sensitive(false);
    let network = ndl::ActionRow::builder().title(tr("Network name")).build();
    network.add_prefix(&gtk::Image::from_icon_name("network-workgroup-symbolic"));
    group.set_description(Some(tr("The name other devices see on the network")));
    group.add(&name);
    group.add(&network);
    content.append(&group);

    let saved = Rc::new(RefCell::new(String::new()));
    let loading = Rc::new(Cell::new(true));
    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let Some(proxy) = bus::proxy(gio::BusType::System, HOSTNAME, HOSTNAME_PATH, HOSTNAME).await
        else {
            group.set_description(Some(tr("The name service is not available.")));
            return;
        };
        let sync = {
            let (proxy, name, network, apply, saved, loading) = (
                proxy.clone(),
                name.clone(),
                network.clone(),
                apply.clone(),
                saved.clone(),
                loading.clone(),
            );
            move || {
                let (proxy, name, network, apply, saved, loading) = (
                    proxy.clone(),
                    name.clone(),
                    network.clone(),
                    apply.clone(),
                    saved.clone(),
                    loading.clone(),
                );
                // Read now from the service: the proxy's cache is emptied whenever `hostnamed`
                // exits for being idle, which left the name blank.
                glib::spawn_future_local(async move {
                    let all =
                        bus::properties(&proxy.connection(), HOSTNAME, HOSTNAME_PATH, HOSTNAME)
                            .await
                            .unwrap_or_default();
                    let text = |key: &str| {
                        all.get(key)
                            .and_then(|v| v.str().map(str::to_owned))
                            .unwrap_or_default()
                    };
                    let pretty = text("PrettyHostname");
                    let host = text("Hostname");
                    let shown = if pretty.is_empty() {
                        host.clone()
                    } else {
                        pretty
                    };
                    if shown.is_empty() {
                        return;
                    }
                    loading.set(true);
                    if *saved.borrow() != shown {
                        name.set_text(&shown);
                        *saved.borrow_mut() = shown;
                    }
                    network.set_subtitle(&host);
                    name.set_sensitive(true);
                    apply.set_sensitive(false);
                    loading.set(false);
                });
            }
        };
        sync();
        bus::on_change(&proxy, {
            let (sync, proxy) = (sync.clone(), proxy.clone());
            move || {
                if proxy.name_owner().is_some() {
                    sync();
                }
            }
        });
        {
            let (saved, loading, apply) = (saved.clone(), loading.clone(), apply.clone());
            name.connect_changed(move |row| {
                if loading.get() {
                    return;
                }
                let text = row.text();
                apply.set_sensitive(!static_hostname(&text).is_empty() && text != *saved.borrow());
            });
        }
        let name2 = name.clone();
        apply.connect_clicked(move |_| {
            let pretty = name2.text().trim().to_string();
            let host = static_hostname(&pretty);
            if host.is_empty() {
                return;
            }
            let (proxy, sync, toasts) = (proxy.clone(), sync.clone(), toasts.clone());
            glib::spawn_future_local(async move {
                let calls = [
                    ("SetPrettyHostname", (pretty, true).to_variant()),
                    ("SetStaticHostname", (host, true).to_variant()),
                ];
                for (method, args) in calls {
                    if let Err(error) = proxy
                        .call_future(method, Some(&args), gio::DBusCallFlags::NONE, -1)
                        .await
                    {
                        toasts.add_toast(ndl::Toast::new(error.message()));
                        break;
                    }
                }
                sync();
            });
        });
    });
    scroller(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_network_name_is_made_of_letters_digits_and_hyphens() {
        assert_eq!(static_hostname("Portátil de Daniel"), "port-til-de-daniel");
        assert_eq!(static_hostname("  PC  "), "pc");
        assert_eq!(static_hostname("--a__b--"), "a-b");
        assert_eq!(static_hostname("***"), "");
    }

    #[test]
    fn the_network_name_is_at_most_63_characters() {
        assert_eq!(static_hostname(&"a".repeat(100)).len(), 63);
    }
}
