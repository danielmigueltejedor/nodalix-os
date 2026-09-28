from __future__ import annotations

import json
import subprocess

from PySide6.QtCore import QObject, Property, Signal, Slot


class InstallerBackend(QObject):
    networkChanged = Signal()
    disksChanged = Signal()
    statusChanged = Signal()

    wifiNetworksChanged = Signal()
    wifiBusyChanged = Signal()
    wifiErrorChanged = Signal()

    settingsChanged = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)

        self._network = "Comprobando…"
        self._disks = []
        self._status = "Preparado"

        self._wifi_networks = []
        self._wifi_busy = False
        self._wifi_error = ""

        self._locale = "es_ES.UTF-8"
        self._timezone = "Europe/Madrid"
        self._keyboard_layout = "es"

        self._locales = self._load_locales()
        self._timezones = self._load_timezones()
        self._keyboard_layouts = self._load_keyboard_layouts()

        self.refresh()

    def run(self, *args):
        return subprocess.run(
            args,
            text=True,
            capture_output=True,
            check=False,
        )

    @Property(str, notify=networkChanged)
    def network(self):
        return self._network

    @Property("QVariantList", notify=disksChanged)
    def disks(self):
        return self._disks

    @Property(str, notify=statusChanged)
    def status(self):
        return self._status

    @Property("QVariantList", notify=wifiNetworksChanged)
    def wifiNetworks(self):
        return self._wifi_networks

    @Property(bool, notify=wifiBusyChanged)
    def wifiBusy(self):
        return self._wifi_busy

    @Property(str, notify=wifiErrorChanged)
    def wifiError(self):
        return self._wifi_error

    @Property("QVariantList", constant=True)
    def locales(self):
        return self._locales

    @Property("QVariantList", constant=True)
    def timezones(self):
        return self._timezones

    @Property("QVariantList", constant=True)
    def keyboardLayouts(self):
        return self._keyboard_layouts

    @Property(str, notify=settingsChanged)
    def locale(self):
        return self._locale

    @locale.setter
    def locale(self, value):
        if value and value != self._locale:
            self._locale = value
            self.settingsChanged.emit()

    @Property(str, notify=settingsChanged)
    def timezone(self):
        return self._timezone

    @timezone.setter
    def timezone(self, value):
        if value and value != self._timezone:
            self._timezone = value
            self.settingsChanged.emit()

    @Property(str, notify=settingsChanged)
    def keyboardLayout(self):
        return self._keyboard_layout

    @keyboardLayout.setter
    def keyboardLayout(self, value):
        if value and value != self._keyboard_layout:
            self._keyboard_layout = value
            self.settingsChanged.emit()

    @Slot()
    def refresh(self):
        self.refresh_network()
        self.refresh_disks()

    @Slot()
    def refresh_network(self):
        result = self.run("nmcli", "-t", "-f", "STATE", "general")
        state = result.stdout.strip().lower()

        if state.startswith("connected"):
            self._network = "Conectado"
        elif state.startswith("connecting"):
            self._network = "Conectando…"
        else:
            self._network = "Sin conexión"

        self.networkChanged.emit()

    @Slot()
    def refresh_wifi(self):
        self._wifi_busy = True
        self._wifi_error = ""

        self.wifiBusyChanged.emit()
        self.wifiErrorChanged.emit()

        devices = self.run(
            "nmcli",
            "-t",
            "-f",
            "DEVICE,TYPE",
            "device",
            "status",
        )

        if ":wifi" not in devices.stdout:
            self._wifi_networks = []
            self._wifi_error = "No se ha detectado ningún adaptador Wi-Fi."

            self.wifiNetworksChanged.emit()
            self.wifiErrorChanged.emit()

            self._wifi_busy = False
            self.wifiBusyChanged.emit()
            return

        self.run("nmcli", "device", "wifi", "rescan")

        result = self.run(
            "nmcli",
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
        )

        networks = {}

        for line in result.stdout.splitlines():

            fields = line.split(":", 3)
            if len(fields) != 4:
                continue

            active, ssid, signal, security = fields
            ssid = ssid.strip()

            if not ssid:
                continue

            try:
                strength = int(signal)
            except ValueError:
                strength = 0

            network = {
                "ssid": ssid,
                "signal": strength,
                "security": security.strip(),
                "secured": bool(security.strip() and security.strip() != "--"),
                "active": active.strip() == "*",
            }

            previous = networks.get(ssid)

            if previous is None or strength > previous["signal"]:
                networks[ssid] = network

        self._wifi_networks = sorted(
            networks.values(),
            key=lambda item: (
                not item["active"],
                -item["signal"],
                item["ssid"].lower(),
            ),
        )

        self.wifiNetworksChanged.emit()

        self._wifi_busy = False
        self.wifiBusyChanged.emit()

    @Slot(str, str)
    def connect_wifi(self, ssid, password):
        ssid = ssid.strip()

        if not ssid:
            return

        self._wifi_busy = True
        self._wifi_error = ""

        self.wifiBusyChanged.emit()
        self.wifiErrorChanged.emit()

        args = [
            "nmcli",
            "device",
            "wifi",
            "connect",
            ssid,
        ]

        if password:
            args += ["password", password]

        result = self.run(*args)

        if result.returncode != 0:
            self._wifi_error = (
                result.stderr.strip()
                or result.stdout.strip()
                or "No se pudo conectar a la red Wi-Fi."
            )
            self.wifiErrorChanged.emit()

        self._wifi_busy = False
        self.wifiBusyChanged.emit()

        self.refresh_network()
        self.refresh_wifi()

    @Slot()
    def refresh_disks(self):
        result = self.run(
            "lsblk",
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,TYPE,MODEL",
        )

        disks = []

        if result.returncode == 0:
            data = json.loads(result.stdout)

            for device in data.get("blockdevices", []):
                if device.get("type") != "disk":
                    continue

                size = int(device.get("size") or 0)

                disks.append(
                    {
                        "name": device.get("name", ""),
                        "path": f"/dev/{device.get('name', '')}",
                        "model": device.get("model") or "Disco",
                        "size": self.format_size(size),
                    }
                )

        self._disks = disks
        self.disksChanged.emit()

    def _load_locales(self):
        result = self.run("localectl", "list-locales")

        locales = [
            value.strip()
            for value in result.stdout.splitlines()
            if value.strip()
        ]

        defaults = [
            "es_ES.UTF-8",
            "en_US.UTF-8",
            "en_GB.UTF-8",
            "fr_FR.UTF-8",
            "de_DE.UTF-8",
            "it_IT.UTF-8",
            "pt_PT.UTF-8",
        ]

        return sorted(set(locales + defaults))

    def _load_timezones(self):
        result = self.run("timedatectl", "list-timezones")

        zones = [
            value.strip()
            for value in result.stdout.splitlines()
            if value.strip()
        ]

        if "Europe/Madrid" not in zones:
            zones.append("Europe/Madrid")

        return sorted(set(zones))

    def _load_keyboard_layouts(self):
        result = self.run(
            "localectl",
            "list-x11-keymap-layouts",
        )

        layouts = [
            value.strip()
            for value in result.stdout.splitlines()
            if value.strip()
        ]

        if not layouts:
            layouts = [
                "es",
                "us",
                "gb",
                "fr",
                "de",
                "it",
                "pt",
            ]

        return sorted(set(layouts))

    def format_size(self, size):
        value = float(size)

        for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
            if value < 1024:
                return f"{value:.1f} {unit}"

            value /= 1024

        return f"{value:.1f} PiB"

    @Slot(str)
    def setStatus(self, status):
        self._status = status
        self.statusChanged.emit()
