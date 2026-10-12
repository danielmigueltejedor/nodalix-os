#!/usr/bin/env python3
"""Nodalix LocalSend context menu for Nautilus."""

from __future__ import annotations

import json
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path
from typing import Iterable

import gi

try:
    gi.require_version("Nautilus", "4.1")
except ValueError:
    gi.require_version("Nautilus", "4.0")
from gi.repository import Gio, GLib, GObject, Nautilus  # noqa: E402


BUS_NAME = "com.nodalix.LocalSend"
OBJECT_PATH = "/com/nodalix/LocalSend"
INTERFACE = "com.nodalix.LocalSend1"
REQUEST_TIMEOUT = 0.35
SEND_HELPER = "/usr/lib/nodalix/localsend-send"
NOTIFICATION_ICON = "send-to-symbolic"


def request(path: str, payload: dict | None = None, timeout: float = REQUEST_TIMEOUT):
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    method = 'GetStatus' if path == '/status' else 'Refresh'
    try:
        result = bus.call_sync(BUS_NAME, OBJECT_PATH, INTERFACE, method, None,
                              None, Gio.DBusCallFlags.NONE, max(1, int(timeout * 1000)), None)
        return json.loads(result.unpack()[0]) if method == 'GetStatus' else {}
    except GLib.Error as error:
        raise OSError(str(error)) from error


def notify(title: str, body: str, urgency: str = "normal") -> None:
    try:
        subprocess.run(
            ["notify-send", "--app-name=Nodalix LocalSend", f"--icon={NOTIFICATION_ICON}",
             f"--urgency={urgency}", title, body],
            check=False,
            timeout=4,
        )
    except (OSError, subprocess.SubprocessError):
        pass


class NodalixLocalSendMenu(GObject.GObject, Nautilus.MenuProvider):
    """Expose nearby LocalSend devices as a native Nautilus submenu."""

    CACHE_SECONDS = 8

    def __init__(self) -> None:
        super().__init__()
        self._devices: list[dict] = []
        self._cache_time = 0.0
        self._refreshing = False
        self._cache_lock = threading.Lock()
        # Nautilus loads the provider before the first context menu. Warm the
        # cache immediately so opening the menu never waits on D-Bus I/O.
        self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self._changed_subscription = self._bus.signal_subscribe(
            BUS_NAME, INTERFACE, 'Changed', OBJECT_PATH, None,
            Gio.DBusSignalFlags.NONE, self._service_changed)
        self._owner_subscription = self._bus.signal_subscribe(
            'org.freedesktop.DBus', 'org.freedesktop.DBus', 'NameOwnerChanged',
            '/org/freedesktop/DBus', BUS_NAME, Gio.DBusSignalFlags.NONE,
            self._service_changed)
        self._refresh_devices_async(force=True)

    def _service_changed(self, *_args) -> None:
        self._refresh_devices_async(force=True)

    def _refresh_devices_async(self, force: bool = False) -> None:
        with self._cache_lock:
            fresh = time.monotonic() - self._cache_time < self.CACHE_SECONDS
            if self._refreshing or (fresh and not force):
                return
            self._refreshing = True

        def refresh() -> None:
            devices: list[dict] | None = None
            try:
                status = request("/status", timeout=2) or {}
                devices = list(status.get("devices") or []) if status.get("enabled") else []
            except (OSError, ValueError, urllib.error.URLError):
                pass
            finally:
                with self._cache_lock:
                    changed = devices is not None and devices != self._devices
                    if devices is not None:
                        self._devices = devices
                        self._cache_time = time.monotonic()
                    self._refreshing = False
                if changed:
                    GLib.idle_add(self._items_changed)

        threading.Thread(
            target=refresh,
            name="nodalix-localsend-menu-refresh",
            daemon=True,
        ).start()

    def _items_changed(self):
        # Nautilus must invalidate an already-open empty menu when discovery
        # completes. Blocking Shell D-Bus on the menu thread caused timeouts.
        self.emit_items_updated_signal()
        return GLib.SOURCE_REMOVE

    def _current_devices(self) -> list[dict]:
        self._refresh_devices_async()
        with self._cache_lock:
            return [device.copy() for device in self._devices]

    def get_file_items(self, files: Iterable[Nautilus.FileInfo]):
        paths: list[str] = []
        for item in files:
            location = item.get_location()
            path = location.get_path() if location is not None else None
            if path and Path(path).exists():
                paths.append(path)

        if not paths:
            return []

        root = Nautilus.MenuItem(
            name="NodalixLocalSend::Share",
            label="Compartir con LocalSend",
            tip="Enviar directamente a un dispositivo cercano",
            icon="send-to-symbolic",
        )
        submenu = Nautilus.Menu()

        # Discovery signals update the cache and invalidate Nautilus menus.
        # No compositor request blocks the Nautilus menu thread.
        devices = self._current_devices()

        if devices:
            for index, device in enumerate(devices):
                fingerprint = str(device.get("fingerprint") or "")
                alias = str(device.get("alias") or "Dispositivo LocalSend")
                model = str(device.get("deviceModel") or "").strip()
                label = alias if not model or model.casefold() == alias.casefold() else f"{alias} · {model}"
                if device.get("favorite"):
                    label = "★ " + label
                receiver = Nautilus.MenuItem(
                    name=f"NodalixLocalSend::Device{index}",
                    label=label,
                    tip=f"Enviar la selección a {alias}",
                    icon="send-to-symbolic",
                )
                receiver.connect("activate", self._queue_send, fingerprint, alias, tuple(paths))
                submenu.append_item(receiver)
        else:
            empty = Nautilus.MenuItem(
                name="NodalixLocalSend::NoDevices",
                label="No hay dispositivos disponibles",
                tip="Abre LocalSend en el dispositivo receptor y vuelve a intentarlo",
            )
            empty.props.sensitive = False
            submenu.append_item(empty)

            refresh = Nautilus.MenuItem(
                name="NodalixLocalSend::Refresh",
                label="Buscar dispositivos",
                tip="Anunciar Nodalix de nuevo en la red local",
            )
            refresh.connect("activate", self._refresh)
            submenu.append_item(refresh)

        settings_item = Nautilus.MenuItem(
            name="NodalixLocalSend::Settings",
            label="Ajustes de LocalSend",
            tip="Configurar el mismo LocalSend del sistema",
            icon="preferences-system-symbolic",
        )
        settings_item.connect("activate", self._open_settings)
        submenu.append_item(settings_item)
        root.set_submenu(submenu)
        return [root]

    def _queue_send(self, _item, fingerprint: str, alias: str, paths: tuple[str, ...]) -> None:
        helper = [
            SEND_HELPER,
            "--fingerprint", fingerprint,
            "--alias", alias,
            "--",
            *paths,
        ]
        # A transient user unit is independent from Nautilus.  The previous
        # in-process daemon thread could disappear immediately after the menu
        # callback and leave the notification stuck at “Preparando envío”.
        unit = f"nodalix-localsend-send-{uuid.uuid4().hex[:12]}"
        command = [
            "systemd-run", "--user", "--collect", "--quiet",
            f"--unit={unit}", "--", *helper,
        ]
        try:
            started = subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
                timeout=4,
            )
            if started.returncode != 0:
                raise RuntimeError(started.stderr.strip() or "No se pudo iniciar el envío")
        except (OSError, RuntimeError, subprocess.SubprocessError):
            # Keep a direct detached fallback for systems without systemd-run.
            try:
                subprocess.Popen(
                    helper,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True,
                )
            except OSError as error:
                notify("No se pudo iniciar el envío", str(error), "critical")

    def _open_settings(self, _item) -> None:
        try:
            subprocess.Popen(["gnome-control-center", "sharing", "localsend"],
                             stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL, start_new_session=True)
        except OSError as error:
            notify("No se pudieron abrir los ajustes de LocalSend", str(error))

    def _refresh(self, _item) -> None:
        def announce() -> None:
            try:
                request("/announce", {}, timeout=2)
                # Allow multicast replies to arrive, then refresh the submenu
                # cache without blocking Nautilus.
                time.sleep(0.7)
                self._refresh_devices_async(force=True)
                notify("Buscando dispositivos", "Vuelve a abrir el menú en unos segundos")
            except (OSError, ValueError, urllib.error.URLError):
                notify("LocalSend no está disponible", "El servicio de Nodalix no está respondiendo", "critical")

        threading.Thread(target=announce, name="nodalix-localsend-refresh", daemon=True).start()
