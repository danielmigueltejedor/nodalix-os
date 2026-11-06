"""Flatpak permission descriptions.

NXPM shows the permissions an application already declares and can remove
local overrides. It does not add access to the whole system, the home
directory or every device.
"""

from __future__ import annotations

import re
from pathlib import Path

from nxpm.errors import NxpmError

_APP_ID = re.compile(r"^[A-Za-z0-9._-]+$")
_BROAD_FILES = {"host", "host-os", "host-etc", "home", "~", "/"}
_BROAD_DEVICES = {"all"}
_BROAD_SOCKETS = {"session-bus", "system-bus"}


def classify(kind: str, value: str) -> str:
    token = value[1:] if value.startswith("!") else value
    base = token.split(":", 1)[0]
    if kind == "filesystems":
        if base in _BROAD_FILES or base.startswith(("home", "~", "/")):
            return "high"
        if base.startswith("xdg-"):
            return "medium"
        return "medium"
    if kind == "devices":
        if base in _BROAD_DEVICES:
            return "high"
        if base == "dri":
            return "low"
        return "medium"
    if kind == "sockets":
        if base in _BROAD_SOCKETS:
            return "high"
        return "low"
    if kind == "shared":
        return "low"
    if kind == "bus":
        return "high" if token.endswith(".*") else "medium"
    return "medium"


def parse_metadata(text: str, origin: str) -> list[dict]:
    rows: list[dict] = []
    section = ""
    for raw in (text or "").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            continue
        if "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        key = key.strip()
        if section == "Context" and key in {"shared", "sockets", "devices", "filesystems"}:
            for item in _items(raw_value):
                rows.append(_row(origin, key, item))
        elif section in {"Session Bus Policy", "System Bus Policy"}:
            rows.append(_row(origin, "bus", f"{key}={raw_value.strip()}"))
    return rows


def user_override_path(app_id: str, overrides_dir: Path) -> Path:
    if not _APP_ID.match(app_id or ""):
        raise NxpmError("invalid-input", "The Flatpak application id is not valid.")
    root = overrides_dir.resolve()
    path = (root / app_id).resolve()
    if path.parent != root:
        raise NxpmError("invalid-input", "The override path leaves the Flatpak overrides directory.")
    return path


def reset_override(app_id: str, overrides_dir: Path) -> bool:
    path = user_override_path(app_id, overrides_dir)
    if path.is_file():
        path.unlink()
        return True
    return False


def _items(value: str) -> list[str]:
    return [item for item in value.strip().strip(";").split(";") if item]


def _row(origin: str, kind: str, value: str) -> dict:
    risk = classify(kind, value)
    if risk == "high":
        summary = "Declared by the application. NXPM does not grant this automatically."
    else:
        summary = "Declared by the application."
    if origin == "override":
        summary = "Added by a local override. Resetting permissions removes it."
    return {
        "id": f"{kind}:{value}",
        "kind": kind,
        "value": value,
        "origin": origin,
        "risk": risk,
        "summary": summary,
    }
