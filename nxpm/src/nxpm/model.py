"""Package identity and records passed across the API."""

from __future__ import annotations

import re
from dataclasses import asdict, dataclass

_FLATPAK_ID = re.compile(
    r"^flatpak:(user|system):(app|runtime)/[A-Za-z0-9._-]+/[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$"
)
_PACMAN_ID = re.compile(r"^pacman:[A-Za-z0-9@._+-]+$")
_APPIMAGE_ID = re.compile(r"^appimage:[A-Za-z0-9][A-Za-z0-9._+-]{0,80}$")
_QUERY = re.compile(r"^[\w .@+_:-]{1,200}$", re.UNICODE)


@dataclass
class Package:
    id: str
    name: str
    source: str
    summary: str = ""
    version: str = ""
    available_version: str = ""
    kind: str = "package"
    installed: bool = False
    update: bool = False
    appstream_id: str = ""
    icon_name: str = ""
    size: int = 0
    risk: str = "low"
    preferred: bool = False
    group_id: str = ""

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class TransactionRecord:
    id: str
    operation: str
    package_id: str
    source: str
    state: str
    progress: float = 0.0
    message: str = ""
    error: str = ""
    cancellable: bool = False

    def to_dict(self) -> dict:
        return asdict(self)


def validate_query(query: str) -> str:
    text = (query or "").strip()
    if not _QUERY.fullmatch(text):
        from nxpm.errors import NxpmError

        raise NxpmError("invalid-input", "The search text contains unsupported characters.")
    return text


def parse_package_id(package_id: str) -> tuple[str, str]:
    """Return ``(source, payload)`` for a validated package id."""

    from nxpm.errors import NxpmError

    if _FLATPAK_ID.fullmatch(package_id or ""):
        _source, scope, ref = package_id.split(":", 2)
        return "flatpak", f"{scope}:{ref}"
    if _PACMAN_ID.fullmatch(package_id or ""):
        return "pacman", package_id.split(":", 1)[1]
    if _APPIMAGE_ID.fullmatch(package_id or ""):
        return "appimage", package_id.split(":", 1)[1]
    raise NxpmError("invalid-input", "The package identifier is not valid.")


def normalize_appstream_id(value: str) -> str:
    text = (value or "").strip()
    if text.endswith(".desktop"):
        text = text[: -len(".desktop")]
    return text
