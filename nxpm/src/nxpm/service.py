"""Session D-Bus service. Methods return before a Flatpak transaction finishes."""

from __future__ import annotations

import logging
import os
import time
from pathlib import Path

from nxpm import API_VERSION
from nxpm.engine import Engine, build_engine
from nxpm.errors import NxpmError

log = logging.getLogger("nxpm.service")
IFACE = "os.nodalix.Nxpm1"
PATH = "/os/nodalix/Nxpm"
BUS_NAME = "os.nodalix.Nxpm"
# The session bus restarts the service on the next call, so a daemon that has
# nothing to remember can leave memory after a quiet period.
IDLE_EXIT_SECONDS = 600
IDLE_CHECK_SECONDS = 60


def activation_available() -> bool:
    """True when the session bus can restart this service on demand."""
    home = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local/share")
    system = (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")
    return any(
        Path(base, "dbus-1", "services", f"{BUS_NAME}.service").is_file()
        for base in [home, *system] if base
    )


class IdleWatch:
    """Decides when the service may exit without losing anything.

    Transaction history only lives in memory, so a service that has run any
    transaction stays resident to keep answering GetHistory/GetTransaction.
    """

    def __init__(self, transactions, timeout: float = IDLE_EXIT_SECONDS, clock=time.monotonic) -> None:
        self.transactions = transactions
        self.timeout = timeout
        self.clock = clock
        self.last_call = clock()

    def touch(self) -> None:
        self.last_call = self.clock()

    def expired(self) -> bool:
        if self.transactions.busy() or self.transactions.history(1):
            return False
        return self.clock() - self.last_call >= self.timeout


def variant_dict(data: dict):
    import dbus

    encoded = {}
    for key, value in data.items():
        if isinstance(value, bool):
            encoded[key] = dbus.Boolean(value)
        elif isinstance(value, float):
            encoded[key] = dbus.Double(value)
        elif isinstance(value, int) and key == "size":
            encoded[key] = dbus.UInt64(max(value, 0))
        else:
            encoded[key] = dbus.String("" if value is None else str(value))
    return dbus.Dictionary(encoded, signature="sv")


def variant_list(rows: list[dict]):
    import dbus

    return dbus.Array([variant_dict(row) for row in rows], signature="a{sv}")


class _DBusError(Exception):
    def __init__(self, name: str, message: str) -> None:
        super().__init__(message)
        self._dbus_error_name = name


def dbus_error(exc: NxpmError) -> _DBusError:
    return _DBusError(exc.dbus_name, exc.message)


def serve(engine: Engine | None = None) -> int:
    import dbus
    import dbus.mainloop.glib
    import dbus.service
    from gi.repository import GLib

    mainloop = dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus(mainloop=mainloop)
    try:
        name = dbus.service.BusName(BUS_NAME, bus, do_not_queue=True)
    except dbus.NameExistsException:
        log.error("NXPM is already running")
        return 1

    active = engine or build_engine()
    active.transactions.defer = True
    watch = IdleWatch(active.transactions)
    service = NxpmObject(bus, active, watch)
    service.bus_name = name
    log.info("NXPM API %s is listening on %s", API_VERSION, BUS_NAME)
    loop = GLib.MainLoop()

    def exit_when_idle() -> bool:
        if watch.expired():
            log.info("NXPM is idle; exiting until the next D-Bus call")
            loop.quit()
            return False
        return True

    # A daemon started by hand without a D-Bus activation file would not come
    # back after exiting, so only an activatable service may leave when idle.
    if activation_available():
        GLib.timeout_add_seconds(IDLE_CHECK_SECONDS, exit_when_idle)
    loop.run()
    return 0


def _package_rows(packages) -> list[dict]:
    return [package.to_dict() for package in packages]


class NxpmObject:
    def __init__(self, bus, engine: Engine, watch: IdleWatch | None = None) -> None:
        import dbus.service

        class Object(dbus.service.Object):
            def __init__(self, bus_connection, path, engine: Engine) -> None:
                super().__init__(bus_connection, path)
                self.engine = engine
                engine.transactions.add_listener(self._changed)

            def _message_cb(self, connection, message):
                if watch is not None:
                    watch.touch()
                return super()._message_cb(connection, message)

            def _changed(self, transaction) -> None:
                payload = (
                    transaction.to_dict()
                    if hasattr(transaction, "to_dict") and not hasattr(transaction, "record")
                    else transaction.record().to_dict()
                )
                self.TransactionChanged(variant_dict(payload))

            @dbus.service.signal(IFACE, signature="a{sv}")
            def TransactionChanged(self, transaction):
                return transaction

            @dbus.service.method(IFACE, in_signature="", out_signature="u")
            def ApiVersion(self):
                return API_VERSION

            @dbus.service.method(IFACE, in_signature="", out_signature="aa{sv}")
            def ListSources(self):
                return variant_list(self.engine.sources())

            @dbus.service.method(IFACE, in_signature="s", out_signature="aa{sv}")
            def ListInstalled(self, source):
                try:
                    return variant_list(_package_rows(self.engine.list_installed(str(source))))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="aa{sv}")
            def Search(self, query):
                try:
                    return variant_list(_package_rows(self.engine.search(str(query))))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="a{sv}")
            def GetDetails(self, package_id):
                try:
                    return variant_dict(self.engine.details(str(package_id)).to_dict())
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="", out_signature="aa{sv}")
            def ListUpdates(self):
                try:
                    return variant_list(_package_rows(self.engine.list_updates()))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="ss", out_signature="s")
            def Install(self, package_id, source):
                try:
                    return self.engine.install(str(package_id), str(source)).id
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="ss", out_signature="s")
            def Remove(self, package_id, source):
                try:
                    return self.engine.remove(str(package_id), str(source)).id
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="ss", out_signature="s")
            def Import(self, kind, path):
                try:
                    return self.engine.import_file(str(kind), str(path)).id
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="aa{sv}")
            def GetPermissions(self, package_id):
                try:
                    return variant_list(self.engine.permissions(str(package_id)))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="aa{sv}")
            def ResetPermissions(self, package_id):
                try:
                    return variant_list(self.engine.reset_permissions(str(package_id)))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="s")
            def Update(self, package_id):
                try:
                    return self.engine.update(str(package_id)).id
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="", out_signature="s")
            def UpgradeSystem(self):
                try:
                    return self.engine.upgrade_system().id
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="a{sv}")
            def GetTransaction(self, transaction_id):
                try:
                    return variant_dict(self.engine.transactions.get(str(transaction_id)).record().to_dict())
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="s", out_signature="b")
            def Cancel(self, transaction_id):
                try:
                    return self.engine.transactions.cancel(str(transaction_id))
                except NxpmError as exc:
                    raise dbus_error(exc) from exc

            @dbus.service.method(IFACE, in_signature="u", out_signature="aa{sv}")
            def GetHistory(self, limit):
                return variant_list(self.engine.transactions.history(int(limit)))

        self._object = Object(bus, PATH, engine)
