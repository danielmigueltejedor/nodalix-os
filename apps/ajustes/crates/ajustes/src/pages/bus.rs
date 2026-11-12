//! D-Bus helpers for the pages that talk to system services (power profiles, time, hostname).

use nodalia_gtk::gtk::{
    gio::{self, prelude::*},
    glib,
};

/// A proxy for a service, or `None` when it is not there. Properties are cached by the proxy.
pub async fn proxy(
    bus: gio::BusType,
    name: &str,
    path: &str,
    interface: &str,
) -> Option<gio::DBusProxy> {
    gio::DBusProxy::for_bus_future(bus, gio::DBusProxyFlags::NONE, None, name, path, interface)
        .await
        .ok()
}

pub fn string_property(proxy: &gio::DBusProxy, name: &str) -> Option<String> {
    proxy
        .cached_property(name)
        .and_then(|v| v.str().map(str::to_owned))
}

pub fn bool_property(proxy: &gio::DBusProxy, name: &str) -> Option<bool> {
    proxy.cached_property(name).and_then(|v| v.get::<bool>())
}

/// Sets a property through `org.freedesktop.DBus.Properties`, which is what services that
/// expose writable properties expect.
pub async fn set_property(
    proxy: &gio::DBusProxy,
    interface: &str,
    name: &str,
    value: &glib::Variant,
) -> Result<(), glib::Error> {
    let parameters = glib::Variant::tuple_from_iter([
        interface.to_variant(),
        name.to_variant(),
        glib::Variant::from_variant(value),
    ]);
    proxy
        .connection()
        .call_future(
            proxy.name().as_deref(),
            proxy.object_path().as_str(),
            "org.freedesktop.DBus.Properties",
            "Set",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            -1,
        )
        .await
        .map(|_| ())
}

/// Runs `f` whenever the service changes a property. The proxy's own signal wants a closure that
/// may run on any thread; this one runs on the main loop, where the widgets are.
pub fn on_change(proxy: &gio::DBusProxy, f: impl Fn() + 'static) {
    proxy.connect_local("g-properties-changed", false, move |_| {
        f();
        None
    });
}
