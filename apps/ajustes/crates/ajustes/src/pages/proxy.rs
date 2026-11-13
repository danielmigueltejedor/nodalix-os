//! The network proxy, from GNOME's `org.gnome.system.proxy`: none, a configuration address
//! (automatic) or a host and port for each kind of connection (manual).

use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

use super::gs::{self, s};
use crate::i18n::tr;

const MODES: [(&str, gs::Val); 3] = [
    s("No proxy", "none"),
    s("Automatic", "auto"),
    s("Manual", "manual"),
];

/// The kinds of connection a manual proxy can be set for: the schema, and the titles of its host
/// and port.
const KINDS: [(&str, &str, &str); 3] = [
    ("org.gnome.system.proxy.http", "HTTP host", "HTTP port"),
    ("org.gnome.system.proxy.https", "HTTPS host", "HTTPS port"),
    ("org.gnome.system.proxy.socks", "SOCKS host", "SOCKS port"),
];

/// A text box for a string key, kept in step with it.
fn text_row(
    settings: &gs::Gs,
    key: &'static str,
    title: &str,
    icon: &str,
) -> Option<ndl::EntryRow> {
    if !settings.has(key) {
        return None;
    }
    let row = ndl::EntryRow::builder().title(title).build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    row.set_text(&settings.settings().string(key));
    row.set_sensitive(settings.settings().is_writable(key));
    let store = settings.settings().clone();
    row.connect_changed(move |row| {
        let text = row.text();
        if store.string(key) != text.as_str() {
            let _ = store.set_string(key, text.as_str());
        }
    });
    Some(row)
}

/// The groups of the proxy settings, or none if the system has no such settings. The groups that
/// only matter for a mode appear while that mode is chosen.
pub fn groups() -> Vec<ndl::PreferencesGroup> {
    let Some(proxy) = gs::open("org.gnome.system.proxy") else {
        return Vec::new();
    };
    let Some(mode) = proxy.choice("mode", tr("Proxy"), &MODES) else {
        return Vec::new();
    };
    mode.add_prefix(&gtk::Image::from_icon_name("network-workgroup-symbolic"));
    let top = ndl::PreferencesGroup::builder()
        .title(tr("Network proxy"))
        .build();
    top.add(&mode);
    let automatic = text_row(
        &proxy,
        "autoconfig-url",
        tr("Configuration address"),
        "network-server-symbolic",
    );
    if let Some(url) = &automatic {
        top.add(url);
    }

    let manual = ndl::PreferencesGroup::builder()
        .title(tr("Manual proxy"))
        .build();
    let mut any = false;
    for (schema, host_title, port_title) in KINDS {
        let Some(kind) = gs::open(schema) else {
            continue;
        };
        if let Some(host) = text_row(&kind, "host", tr(host_title), "network-server-symbolic") {
            manual.add(&host);
            any = true;
        }
        if let Some(port) = kind.spin("port", tr(port_title), (0.0, 65535.0, 1.0)) {
            port.add_css_class("numeric");
            manual.add(&port);
        }
    }

    let show = {
        let (store, automatic, manual) = (proxy.settings().clone(), automatic, manual.clone());
        move || {
            let current = store.string("mode");
            if let Some(url) = &automatic {
                url.set_visible(current == "auto");
            }
            manual.set_visible(any && current == "manual");
        }
    };
    show();
    proxy
        .settings()
        .connect_changed(Some("mode"), move |_, _| show());
    vec![top, manual]
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_text_is_translated() {
        for text in [
            "Network proxy",
            "Proxy",
            "No proxy",
            "Automatic",
            "Manual",
            "Configuration address",
            "Manual proxy",
            "HTTP host",
            "HTTP port",
            "HTTPS host",
            "HTTPS port",
            "SOCKS host",
            "SOCKS port",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
    }
}
