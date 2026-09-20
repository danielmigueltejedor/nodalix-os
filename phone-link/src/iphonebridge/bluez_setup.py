"""BlueZ adapter prep — the toggle-dance from spike/RESULTS.md §1.

For MAP/PBAP to be reachable on iOS 26.5, three things must be true on
the Linux side:

1. Adapter Class-of-Device set to A/V Hands-Free (Major=4 Minor=8).
2. A BLE peripheral advert is active with SolicitUUIDs containing the
   ANCS UUID. Without this, the iPhone never surfaces the per-device
   "Show Message Notifications" / "Sync Contacts" toggles.
3. Adapter is powered.

This module owns those three concerns. Re-run safely on startup.

CoD setting requires CAP_NET_ADMIN (essentially root), so we shell out to
sudo btmgmt unless we detect we already have it. The BLE advert is
user-bus DBus and needs no privileges.
"""
from __future__ import annotations

import logging
import os
import subprocess
from typing import Any

import dbus
import dbus.exceptions
import dbus.service

from iphonebridge import config
from iphonebridge.bus import bluez, system_bus

log = logging.getLogger(__name__)


# ---- Adapter discovery --------------------------------------------------

def resolve_adapter() -> str:
    """Return a currently available BlueZ adapter name.

    Bluetooth USB firmware recovery can make the kernel re-enumerate the same
    controller as hci1 (or higher) without rebooting.  The original project
    hard-coded hci0, which made the daemon crash-loop after such a recovery.
    Prefer the configured adapter when it exists, otherwise select the first
    live Adapter1 object and update the process-wide configuration.
    """
    om = dbus.Interface(
        system_bus.get_object("org.bluez", "/"),
        "org.freedesktop.DBus.ObjectManager",
    )
    managed = om.GetManagedObjects()
    wanted_path = f"/org/bluez/{config.ADAPTER}"
    if "org.bluez.Adapter1" in managed.get(wanted_path, {}):
        return config.ADAPTER

    adapters = sorted(
        str(path).rsplit("/", 1)[-1]
        for path, ifaces in managed.items()
        if "org.bluez.Adapter1" in ifaces
    )
    if not adapters:
        raise dbus.exceptions.DBusException(
            "No Bluetooth adapter is currently available",
            name="org.bluez.Error.NotReady",
        )
    previous = config.ADAPTER
    config.ADAPTER = adapters[0]
    log.warning("Bluetooth adapter changed from %s to %s", previous, config.ADAPTER)
    return config.ADAPTER


# ---- Class-of-Device ----------------------------------------------------

def current_cod() -> int | None:
    """Return adapter Class field, or None if unavailable."""
    try:
        v = bluez(f"/org/bluez/{config.ADAPTER}",
                  "org.freedesktop.DBus.Properties").Get(
            "org.bluez.Adapter1", "Class")
        return int(v)
    except dbus.exceptions.DBusException:
        return None


def desired_cod_matches(cod: int | None) -> bool:
    """Major & Minor match what we want? Service-class bits are derived
    by BlueZ from registered profiles, so we only compare the low 16 bits
    of (Major<<8 | Minor<<2)."""
    if cod is None:
        return False
    major = (cod >> 8) & 0x1F
    minor = (cod >> 2) & 0x3F
    return major == config.COD_MAJOR and (minor << 2) == config.COD_MINOR


def set_cod(*, dry_run: bool = False) -> bool:
    """Apply A/V Hands-Free CoD via btmgmt. Returns True on success."""
    index = config.ADAPTER.removeprefix("hci")
    cmd = ["btmgmt", "--index", index, "class",
           str(config.COD_MAJOR), str(config.COD_MINOR)]
    if os.geteuid() != 0:
        cmd = ["sudo", "-n"] + cmd  # non-interactive sudo; user pre-grants
    log.info("setting adapter CoD via: %s", " ".join(cmd))
    if dry_run:
        return True
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        log.error("btmgmt failed: %s", e)
        return False
    if r.returncode != 0:
        log.error("btmgmt class %d %d failed (rc=%d): %s",
                  config.COD_MAJOR, config.COD_MINOR, r.returncode,
                  r.stderr.strip() or r.stdout.strip())
        return False
    log.info("CoD set ok: %s", r.stdout.strip())
    return True


# ---- BLE advertisement (SolicitUUIDs = ANCS) ----------------------------

class _AncsAdvert(dbus.service.Object):
    """Minimal LEAdvertisement1 object so iOS shows our toggles."""

    PATH = config.BLE_ADVERT_DBUS_PATH

    @dbus.service.method("org.bluez.LEAdvertisement1",
                         in_signature="", out_signature="")
    def Release(self) -> None:
        return None

    @dbus.service.method("org.freedesktop.DBus.Properties",
                         in_signature="s", out_signature="a{sv}")
    def GetAll(self, iface: str) -> dict[str, Any]:
        if iface != "org.bluez.LEAdvertisement1":
            raise dbus.exceptions.DBusException(
                f"Unknown interface {iface}",
                name="org.freedesktop.DBus.Error.InvalidArgs")
        properties = {
            "Type": dbus.String("peripheral"),
            "SolicitUUIDs": dbus.Array([config.ANCS_SOLICIT_UUID], signature="s"),
            "Includes": dbus.Array(["tx-power"], signature="s"),
            # Keep the ANCS solicitation connectable/discoverable even when the
            # classic adapter is hidden.  iOS may unpublish ANCS between BLE
            # sessions and otherwise has no reason to recreate the LE link.
            # We deliberately omit LocalName below unless explicitly set, so
            # this does not create another visible "Nodalix" entry.
            "Discoverable": dbus.Boolean(True),
            "DiscoverableTimeout": dbus.UInt16(0),
            "Appearance": dbus.UInt16(0x0080),  # generic computer
        }
        # A second advertised local name makes iOS display the LE endpoint as
        # another Nodalix device next to the already bonded BR/EDR identity.
        # The ANCS solicitation UUID is sufficient; only expose a name when an
        # administrator explicitly configured a non-empty one.
        if config.BLE_ADVERT_LOCAL_NAME:
            properties["LocalName"] = dbus.String(config.BLE_ADVERT_LOCAL_NAME)
        return properties

    @dbus.service.method("org.freedesktop.DBus.Properties",
                         in_signature="ss", out_signature="v")
    def Get(self, iface: str, prop: str):
        return self.GetAll(iface)[prop]


_advert_instance: _AncsAdvert | None = None


def register_advert() -> bool:
    """Register the BLE advertisement on the system bus.

    Idempotent — calling twice is harmless because BlueZ will reject the
    second registration and we treat that as success.
    """
    global _advert_instance
    try:
        if _advert_instance is None:
            _advert_instance = _AncsAdvert(system_bus, _AncsAdvert.PATH)
        ad_mgr = bluez(f"/org/bluez/{config.ADAPTER}", "org.bluez.LEAdvertisingManager1")
        # BlueZ's RegisterAdvertisement frequently NoReply-timeouts even
        # though it actually registers. Retry after NoReply; only an explicit
        # AlreadyExists response confirms that our advertisement is present.
        # python-dbus cannot infer a signature from an empty Python dict when
        # introspection is temporarily unavailable (common just after an HCI
        # firmware reset).  Give BlueZ the explicit a{sv} it expects.
        options = dbus.Dictionary({}, signature="sv")
        ad_mgr.RegisterAdvertisement(_AncsAdvert.PATH, options, timeout=10.0)
        log.info("BLE advert registered: %s", _AncsAdvert.PATH)
        return True
    except dbus.exceptions.DBusException as e:
        name = e.get_dbus_name()
        if name == "org.bluez.Error.AlreadyExists":
            log.info("BLE advert already registered")
            return True
        if name in {"org.bluez.Error.NotPermitted", "org.bluez.Error.Failed",
                    "org.freedesktop.DBus.Error.NoReply"}:
            log.warning("BLE advertising temporarily unavailable (%s); will retry without stopping phone services", name)
            return False
        log.error("RegisterAdvertisement failed: %s: %s",
                  name, e.get_dbus_message())
        return False


def unregister_advert() -> None:
    """Best-effort unregister; safe to call on shutdown."""
    try:
        ad_mgr = bluez(f"/org/bluez/{config.ADAPTER}",
                       "org.bluez.LEAdvertisingManager1")
        ad_mgr.UnregisterAdvertisement(_AncsAdvert.PATH)
    except dbus.exceptions.DBusException as e:
        log.debug("UnregisterAdvertisement: %s", e.get_dbus_name())


# ---- one-shot startup ---------------------------------------------------

def prepare(*, allow_sudo: bool = True) -> bool:
    """Run all the prerequisites. Returns False if anything critical failed.

    Idempotent. Safe to call on every daemon start.
    """
    ok = True
    try:
        resolve_adapter()
    except dbus.exceptions.DBusException as e:
        log.error("No usable Bluetooth adapter: %s", e.get_dbus_message())
        return False
    try:
        cod = current_cod()
    except (dbus.exceptions.DBusException, OSError) as error:
        log.warning("Cannot read Bluetooth adapter state: %s", error)
        return False
    log.info("current adapter Class = 0x%06x", cod or 0)
    if not desired_cod_matches(cod):
        if not allow_sudo and os.geteuid() != 0:
            log.warning("CoD wrong but sudo disabled — skipping CoD set")
        else:
            try:
                ok &= set_cod()
            except (dbus.exceptions.DBusException, OSError) as error:
                log.warning("Cannot prepare Bluetooth class: %s", error)
                ok = False
    else:
        log.info("CoD already matches A/V Hands-Free, leaving as-is")

    ok &= register_advert()
    return ok
