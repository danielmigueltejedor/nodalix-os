"""Runtime policy. Unimplemented sources cannot be switched on."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SourceSpec:
    enabled: bool
    implemented: bool
    reason: str = ""


DEFAULT_SOURCES = {
    "flatpak": SourceSpec(True, True, "Flatpak remotes, including Flathub."),
    "pacman": SourceSpec(True, True, "Official Arch repositories through libalpm."),
    "aur": SourceSpec(
        False,
        False,
        "Disabled. AUR builds are not offered until review and unprivileged builds exist.",
    ),
    "appimage": SourceSpec(
        True,
        True,
        "User applications copied into ~/Applications. Debian packages are unpacked, not installed into the system.",
    ),
}


@dataclass
class Config:
    sources: dict[str, SourceSpec]
    preference: list[str]
    check_updates: bool = True
    notify: bool = True

    def active_sources(self) -> list[str]:
        return [name for name, spec in self.sources.items() if spec.enabled and spec.implemented]


def config_path() -> Path:
    from os import environ

    base = Path(environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    return base / "nxpm" / "config.json"


def load_config(path: Path | None = None) -> Config:
    sources = {
        name: SourceSpec(spec.enabled, spec.implemented, spec.reason)
        for name, spec in DEFAULT_SOURCES.items()
    }
    preference = ["flatpak", "pacman", "appimage"]
    check_updates = True
    notify = True
    location = path if path is not None else config_path()
    if location.is_file():
        try:
            data = json.loads(location.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            data = {}
        if not isinstance(data, dict):
            data = {}
        requested = data.get("sources", {})
        if isinstance(requested, dict):
            for name, spec in sources.items():
                entry = requested.get(name)
                if spec.implemented and isinstance(entry, dict) and "enabled" in entry:
                    spec.enabled = bool(entry["enabled"])
        requested_preference = data.get("preference")
        if isinstance(requested_preference, list):
            cleaned = [item for item in requested_preference if item in sources and sources[item].implemented]
            if cleaned:
                preference = cleaned
        updates = data.get("updates", {})
        if isinstance(updates, dict):
            if "check" in updates:
                check_updates = bool(updates["check"])
            if "notify" in updates:
                notify = bool(updates["notify"])
    return Config(sources, preference, check_updates, notify)
