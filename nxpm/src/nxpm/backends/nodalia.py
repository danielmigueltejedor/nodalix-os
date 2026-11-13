"""The applications of the Nodalia suite, published as AppImages on GitHub.

The catalogue (``nodalia-apps.json``) names each application, the repository
that publishes it and the release asset to install. NXPM installs the asset
into ``~/Applications`` the way it installs any AppImage (same folder, same
menu entry), but it also knows where the application comes from, so the Store
can offer it without the user visiting the web and can update it when the
repository publishes a new release.

Two kinds of entry exist. ``appimage`` entries are downloaded and registered by
NXPM itself. ``nodalix-apps`` entries (Vinilo and the other applications that
Nodalix OS already installs under ``~/.local/opt/nodalix/apps``) are installed
and updated by the ``nodalix-apps`` command, which NXPM only runs; NXPM reads
their installed version and asks GitHub for the latest release.

Nothing here runs the downloaded file. The download goes over HTTPS to the
hosts GitHub serves release assets from, is checked for the AppImage Type-2
marker and, when the release carries a checksum file, against its SHA-256.
"""

from __future__ import annotations

import fnmatch
import json
import os
import re
import subprocess
import time
from pathlib import Path

from nxpm.applications import (
    applications_dir,
    desktop_path,
    load_catalog,
    remove_tree,
    safe_key,
    update_catalog,
    write_desktop,
)
from nxpm.errors import NxpmError
from nxpm.model import Package
from nxpm.updates_appimage import (
    _https_asset,
    _opener,
    _stream_limited,
    look_like_appimage,
)

SYSTEM_CATALOG = "/usr/share/nodalix/nodalia-apps.json"
CACHE_SECONDS = 3600
_SLUG = re.compile(r"^[a-z0-9][a-z0-9-]{0,40}$")
_REPO = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*/[A-Za-z0-9][A-Za-z0-9._-]*$")
_SUMS_LINE = re.compile(r"^([0-9a-fA-F]{64})\s+\*?(.+)$")
_MAX_SUMS = 256 * 1024
_MANAGER_TIMEOUT = 1800


def catalog_location() -> Path:
    override = os.environ.get("NXPM_NODALIA_CATALOG")
    if override:
        return Path(override)
    system = Path(SYSTEM_CATALOG)
    if system.is_file():
        return system
    # A checkout of nodalix-os: nxpm/src/nxpm/backends/nodalia.py -> nxpm/share/.
    return Path(__file__).resolve().parents[3] / "share" / "nodalia-apps.json"


def nodalix_apps_root() -> Path:
    override = os.environ.get("NXPM_NODALIX_APPS_ROOT")
    return Path(override) if override else Path.home() / ".local" / "opt" / "nodalix" / "apps"


def load_entries(path: Path | None = None) -> dict[str, dict]:
    """The catalogue entries that are well formed, by slug."""

    location = path or catalog_location()
    try:
        data = json.loads(location.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeError):
        return {}
    rows = data.get("apps") if isinstance(data, dict) else None
    entries: dict[str, dict] = {}
    for row in rows if isinstance(rows, list) else []:
        if not isinstance(row, dict):
            continue
        slug = str(row.get("id") or "")
        repo = str(row.get("repo") or "")
        kind = str(row.get("kind") or "appimage")
        asset = str(row.get("asset") or "")
        manager = str(row.get("manager") or "")
        if not (_SLUG.match(slug) and _REPO.match(repo)):
            continue
        if kind == "appimage" and not asset.endswith(".AppImage"):
            continue
        if kind == "nodalix-apps" and not _SLUG.match(manager):
            continue
        if kind not in {"appimage", "nodalix-apps"}:
            continue
        entries[slug] = {
            "id": slug,
            "kind": kind,
            "name": str(row.get("name") or slug),
            "summary": str(row.get("summary") or ""),
            "repo": repo,
            "asset": asset,
            "checksums": str(row.get("checksums") or ""),
            "manager": manager,
            "icon": str(row.get("icon") or ""),
        }
    return entries


def release_version(tag: str) -> str:
    text = (tag or "").strip()
    return text[1:] if re.match(r"^v\d", text) else text


class NodaliaBackend:
    source = "nodalia"

    def __init__(self, catalog: Path | None = None, opener=None, clock=time.monotonic) -> None:
        self._catalog = catalog
        self._opener = opener
        self._clock = clock
        self._latest_cache: dict[str, tuple[float, dict | None]] = {}

    # -- listing -----------------------------------------------------------

    def search(self, query: str) -> list[Package]:
        needle = query.casefold()
        return [
            package
            for package in self.list_catalog()
            if needle in package.name.casefold()
            or needle in package.id.casefold()
            or needle in package.summary.casefold()
        ]

    def list_catalog(self) -> list[Package]:
        """Every Nodalia application, installed or not. Offline: uses what is cached."""

        return [self._package(entry, self._cached(entry)) for entry in self._entries().values()]

    def list_installed(self) -> list[Package]:
        return [package for package in self.list_catalog() if package.installed]

    def list_updates(self) -> list[Package]:
        found = []
        for slug, entry in self._entries().items():
            if self._record(slug) is None:
                continue
            package = self._package(entry, self._fetch_latest(entry, quiet=True))
            if package.update:
                found.append(package)
        return found

    def details(self, package_id: str) -> Package:
        slug = _slug(package_id)
        entry = self._entry(slug)
        return self._package(entry, self._fetch_latest(entry, quiet=True))

    # -- changes -----------------------------------------------------------

    def apply(self, package_id: str, operation: str, progress) -> None:
        slug = _slug(package_id)
        entry = self._entry(slug)
        if entry["kind"] == "nodalix-apps":
            self._apply_managed(entry, operation, progress)
            return
        if operation == "remove":
            self._remove(slug)
            progress(1.0, "Removed from Applications")
            return
        if operation == "install" and self._record(slug) is not None:
            raise NxpmError("failed", "That application is already installed.")
        if operation == "update" and self._record(slug) is None:
            raise NxpmError("not-found", "That application is not installed.")
        if operation not in {"install", "update"}:
            raise NxpmError("unsupported", "That operation is not available for Nodalia applications.")
        self._install(slug, entry, operation, progress)

    def _apply_managed(self, entry: dict, operation: str, progress) -> None:
        """Install or update through ``nodalix-apps``, which already knows how."""

        if operation not in {"install", "update"}:
            raise NxpmError(
                "unsupported",
                "Remove this application from the Applications library; NXPM installs and updates it.",
            )
        installed = _managed_record(entry["manager"]) is not None
        if operation == "install" and installed:
            raise NxpmError("failed", "That application is already installed.")
        if operation == "update" and not installed:
            raise NxpmError("not-found", "That application is not installed.")
        progress(0.1, f"{'Installing' if operation == 'install' else 'Updating'} {entry['name']}")
        try:
            completed = subprocess.run(
                ["nodalix-apps", "update", entry["manager"]],
                capture_output=True,
                text=True,
                check=False,
                timeout=_MANAGER_TIMEOUT,
            )
        except FileNotFoundError as exc:
            raise NxpmError("unsupported", "nodalix-apps is not installed on this system.") from exc
        except subprocess.TimeoutExpired as exc:
            raise NxpmError("failed", "The installation took too long and was stopped.") from exc
        if completed.returncode != 0:
            detail = (completed.stderr or completed.stdout or "").strip().splitlines()
            raise NxpmError("failed", detail[-1] if detail else "nodalix-apps could not finish.")
        if _managed_record(entry["manager"]) is None:
            raise NxpmError("failed", "nodalix-apps finished but the application is not installed.")
        progress(1.0, "Installed" if operation == "install" else "Updated")

    def _install(self, slug: str, entry: dict, operation: str, progress) -> None:
        progress(0.05, "Looking up the latest release")
        latest = self._fetch_latest(entry, quiet=False)
        if latest is None:
            raise NxpmError("failed", "The release has no AppImage for this application.")
        record = self._record(slug)
        if operation == "update" and record is not None and record.get("version") == latest["version"]:
            progress(1.0, "Already up to date")
            return
        key = safe_key(slug)
        root = applications_dir()
        destination = root / f"{key}.AppImage"
        temporary = root / f"{key}.AppImage.partial"
        temporary.unlink(missing_ok=True)
        opener = self._opener or _opener()
        try:
            progress(0.1, f"Downloading {entry['name']} {latest['version']}")
            digest = _download(latest["url"], temporary, opener)
            if not look_like_appimage(temporary):
                raise NxpmError("failed", "The published file is not an AppImage NXPM can install.")
            if latest["sums_url"]:
                progress(0.9, "Checking the SHA-256")
                expected = _published_digest(latest["sums_url"], latest["name"], opener)
                if expected != digest:
                    raise NxpmError("failed", "The downloaded AppImage does not match its published SHA-256.")
            temporary.chmod(0o755)
            temporary.replace(destination)
        except Exception:
            temporary.unlink(missing_ok=True)
            raise
        summary = entry["summary"] or "Nodalia application."
        write_desktop(key, entry["name"], destination, summary, entry["icon"])
        data = {
            "name": entry["name"],
            "format": "appimage",
            "file": destination.name,
            "update_info": "",
            "summary": summary,
            "origin": "nodalia",
            "slug": slug,
            "version": latest["version"],
        }
        update_catalog(lambda catalog: catalog["apps"].__setitem__(key, data))
        progress(1.0, "Installed" if operation == "install" else "Updated")

    def _remove(self, slug: str) -> None:
        record = self._record(slug)
        if record is None:
            raise NxpmError("not-found", "That application is not installed.")
        key = safe_key(slug)
        root = applications_dir()
        for name in (str(record.get("file") or f"{key}.AppImage"), f"{key}.AppImage.partial"):
            path = root / name
            if path.exists() or path.is_symlink():
                remove_tree(path, root)
        menu = desktop_path(key)
        if menu.is_file():
            menu.unlink()
        update_catalog(lambda catalog: catalog["apps"].pop(key, None))

    # -- helpers -----------------------------------------------------------

    def _entries(self) -> dict[str, dict]:
        return load_entries(self._catalog)

    def _entry(self, slug: str) -> dict:
        entry = self._entries().get(slug)
        if entry is None:
            raise NxpmError("not-found", "That application is not in the Nodalia catalogue.")
        return entry

    def _record(self, slug: str) -> dict | None:
        """The installed application, or None. Same shape for both kinds."""

        entry = self._entries().get(slug)
        if entry is not None and entry["kind"] == "nodalix-apps":
            return _managed_record(entry["manager"])
        record = load_catalog().get("apps", {}).get(safe_key(slug))
        if isinstance(record, dict) and record.get("origin") == "nodalia":
            return record
        return None

    def _cached(self, entry: dict) -> dict | None:
        cached = self._latest_cache.get(entry["id"])
        return cached[1] if cached is not None else None

    def _fetch_latest(self, entry: dict, quiet: bool) -> dict | None:
        cached = self._latest_cache.get(entry["id"])
        if cached is not None and self._clock() - cached[0] < CACHE_SECONDS:
            return cached[1]
        try:
            latest = _latest_release(entry, self._opener or _opener())
        except NxpmError:
            if quiet:
                # Offline or rate limited: keep what is known, ask again later.
                return cached[1] if cached is not None else None
            raise
        self._latest_cache[entry["id"]] = (self._clock(), latest)
        return latest

    def _package(self, entry: dict, latest: dict | None) -> Package:
        record = self._record(entry["id"])
        installed = record is not None
        version = str(record.get("version") or "") if record else ""
        available = latest["version"] if latest else ""
        return Package(
            id=f"nodalia:{entry['id']}",
            name=entry["name"],
            source="nodalia",
            summary=entry["summary"],
            version=version,
            available_version=available,
            kind="app",
            installed=installed,
            update=bool(installed and available and available != version),
            icon_name=entry["icon"],
            size=int(latest["size"]) if latest else 0,
            risk="low",
            preferred=True,
        )


def _managed_record(manager: str) -> dict | None:
    """What ``nodalix-apps`` has installed for ``manager``, read from its folder."""

    current = nodalix_apps_root() / manager / "current"
    try:
        target = current.resolve(strict=True)
    except OSError:
        return None
    if not current.is_symlink() or not target.is_dir():
        return None
    version_file = target / "version"
    try:
        version = version_file.read_text(encoding="utf-8").strip() if version_file.is_file() else target.name
    except (OSError, UnicodeError):
        version = target.name
    return {"version": release_version(version), "origin": "nodalix-apps"}


def _slug(package_id: str) -> str:
    from nxpm.model import parse_package_id

    source, payload = parse_package_id(package_id)
    if source != "nodalia":
        raise NxpmError("invalid-input", "That identifier is not a Nodalia application.")
    return payload


def _latest_release(entry: dict, opener) -> dict | None:
    """The newest stable release: its version, AppImage URL and checksum URL."""

    import urllib.request

    request = urllib.request.Request(
        f"https://api.github.com/repos/{entry['repo']}/releases/latest",
        headers={"Accept": "application/vnd.github+json", "User-Agent": "NXPM/0.1"},
    )
    try:
        with opener.open(request, timeout=30) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeError) as exc:
        raise NxpmError("failed", "The Nodalia release could not be read from GitHub.") from exc
    if not isinstance(payload, dict):
        raise NxpmError("failed", "GitHub answered with something NXPM does not understand.")
    version = release_version(str(payload.get("tag_name") or ""))
    if entry["kind"] == "nodalix-apps":
        return {"version": version, "sums_url": "", "name": "", "url": "", "size": 0} if version else None
    image = None
    sums_url = ""
    for asset in payload.get("assets") or []:
        if not isinstance(asset, dict):
            continue
        name = str(asset.get("name") or "")
        link = str(asset.get("browser_download_url") or "")
        if not _https_asset(link):
            continue
        if image is None and fnmatch.fnmatch(name, entry["asset"]):
            image = {"name": name, "url": link, "size": int(asset.get("size") or 0)}
        elif entry["checksums"] and name == entry["checksums"]:
            sums_url = link
    if not version or image is None:
        return None
    if entry["checksums"] and not sums_url:
        # A checksum file was promised and is missing: do not install unchecked.
        return None
    return {"version": version, "sums_url": sums_url, **image}


def _download(url: str, destination: Path, opener) -> str:
    import urllib.request

    request = urllib.request.Request(url, headers={"User-Agent": "NXPM/0.1"})
    try:
        with opener.open(request, timeout=60) as response:
            return _stream_limited(response, destination)
    except OSError as exc:
        raise NxpmError("failed", "The AppImage could not be downloaded.") from exc


def _published_digest(url: str, name: str, opener) -> str:
    import urllib.request

    request = urllib.request.Request(url, headers={"User-Agent": "NXPM/0.1"})
    try:
        with opener.open(request, timeout=30) as response:
            text = response.read(_MAX_SUMS + 1)
    except OSError as exc:
        raise NxpmError("failed", "The checksum file could not be downloaded.") from exc
    if len(text) > _MAX_SUMS:
        raise NxpmError("failed", "The checksum file is larger than expected.")
    for line in text.decode("utf-8", errors="replace").splitlines():
        match = _SUMS_LINE.match(line.strip())
        if match and Path(match.group(2)).name == name:
            return match.group(1).lower()
    raise NxpmError("failed", "The release does not list a checksum for this AppImage.")

