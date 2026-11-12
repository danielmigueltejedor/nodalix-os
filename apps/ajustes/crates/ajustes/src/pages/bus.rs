//! D-Bus helpers for the pages that talk to system services (power profiles, time, hostname).

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

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

/// The system bus, or `None` when there is none.
pub async fn system() -> Option<gio::DBusConnection> {
    gio::bus_get_future(gio::BusType::System).await.ok()
}

/// A page's system bus connection and its in-flight refresh.
#[derive(Default)]
pub(super) struct SystemRefresh {
    connection: RefCell<Option<gio::DBusConnection>>,
    pending: Cell<bool>,
}

pub(super) struct RefreshGuard(Rc<SystemRefresh>);

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        self.0.pending.set(false);
    }
}

impl SystemRefresh {
    pub(super) fn cached(&self) -> Option<gio::DBusConnection> {
        self.connection.borrow().clone()
    }

    fn begin(self: &Rc<Self>) -> Option<RefreshGuard> {
        if self.pending.replace(true) {
            return None;
        }
        Some(RefreshGuard(self.clone()))
    }

    pub(super) fn run<F>(
        self: &Rc<Self>,
        refresh: impl FnOnce(Option<gio::DBusConnection>, RefreshGuard) -> F + 'static,
    ) where
        F: Future<Output = ()> + 'static,
    {
        let Some(guard) = self.begin() else {
            return;
        };
        let bus = self.clone();
        glib::spawn_future_local(async move {
            let connection = match bus.cached() {
                Some(connection) => Some(connection),
                None => system().await,
            };
            *bus.connection.borrow_mut() = connection.clone();
            refresh(connection, guard).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_can_restart_only_after_its_guard_is_released() {
        let bus = Rc::new(SystemRefresh::default());
        let guard = bus.begin().expect("the first refresh should start");
        assert!(bus.begin().is_none());
        drop(guard);
        assert!(bus.begin().is_some());
    }
}

/// The session bus, or `None` when there is none.
pub async fn session() -> Option<gio::DBusConnection> {
    gio::bus_get_future(gio::BusType::Session).await.ok()
}

/// Calls a method and returns its reply, giving up after five seconds.
pub async fn call(
    connection: &gio::DBusConnection,
    name: &str,
    path: &str,
    interface: &str,
    method: &str,
    parameters: Option<&glib::Variant>,
) -> Result<glib::Variant, glib::Error> {
    call_for(connection, name, path, interface, method, parameters, 5_000).await
}

/// Like `call`, for a method that may take a while (pairing, connecting): thirty seconds.
pub async fn call_slow(
    connection: &gio::DBusConnection,
    name: &str,
    path: &str,
    interface: &str,
    method: &str,
    parameters: Option<&glib::Variant>,
) -> Result<glib::Variant, glib::Error> {
    call_for(
        connection, name, path, interface, method, parameters, 30_000,
    )
    .await
}

async fn call_for(
    connection: &gio::DBusConnection,
    name: &str,
    path: &str,
    interface: &str,
    method: &str,
    parameters: Option<&glib::Variant>,
    timeout_ms: i32,
) -> Result<glib::Variant, glib::Error> {
    connection
        .call_future(
            Some(name),
            path,
            interface,
            method,
            parameters,
            None,
            gio::DBusCallFlags::NONE,
            timeout_ms,
        )
        .await
}

/// All the properties of an interface of an object.
pub async fn properties(
    connection: &gio::DBusConnection,
    name: &str,
    path: &str,
    interface: &str,
) -> Option<std::collections::HashMap<String, glib::Variant>> {
    let reply = call(
        connection,
        name,
        path,
        "org.freedesktop.DBus.Properties",
        "GetAll",
        Some(&(interface,).to_variant()),
    )
    .await
    .ok()?;
    reply
        .get::<(std::collections::HashMap<String, glib::Variant>,)>()
        .map(|(all,)| all)
}

/// An object path from a string, or `None` if it is not one.
pub fn object_path(path: &str) -> Option<glib::variant::ObjectPath> {
    glib::variant::ObjectPath::try_from(path).ok()
}
