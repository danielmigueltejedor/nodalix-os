"""Applications imported into ~/Applications.

AppImage files are copied there and registered in the user menu. Debian
packages are unpacked into an AppDir without running maintainer scripts.
Updates run only when the AppImage embeds a channel NXPM can check.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from nxpm.applications import (
    applications_dir,
    confine,
    desktop_path,
    load_catalog,
    open_user_file,
    remove_tree,
    safe_key,
    update_catalog,
    write_desktop,
)
from nxpm.elfinfo import is_elf, update_information
from nxpm.errors import NxpmError
from nxpm.foreign import extract_deb
from nxpm.model import Package
from nxpm.updates_appimage import apply_update, describe_channel

_DEB_LIMIT = 1024 * 1024 * 1024


class AppImageBackend:
    source = "appimage"

    def __init__(self, updater=None) -> None:
        self._updater = updater or apply_update

    def search(self, query: str) -> list[Package]:
        needle = query.casefold()
        return [
            package
            for package in self.list_installed()
            if needle in package.name.casefold() or needle in package.id.casefold()
        ]

    def list_installed(self) -> list[Package]:
        packages = []
        for key, record in load_catalog().get("apps", {}).items():
            if isinstance(record, dict):
                packages.append(self._package(str(key), record))
        return packages

    def list_updates(self) -> list[Package]:
        found = []
        for package in self.list_installed():
            record = load_catalog()["apps"].get(package.id.split(":", 1)[1], {})
            if describe_channel(str(record.get("update_info") or "")):
                package.update = True
                package.summary = "An update channel is embedded in this AppImage."
                found.append(package)
        return found

    def details(self, package_id: str) -> Package:
        key = _key(package_id)
        record = load_catalog().get("apps", {}).get(key)
        if not isinstance(record, dict):
            raise NxpmError("not-found", "That application is not in the Applications folder.")
        return self._package(key, record)

    def import_file(self, kind: str, path: str, progress) -> Package:
        if kind == "appimage":
            return self._import_appimage(path, progress)
        if kind == "deb":
            return self._import_deb(path, progress)
        raise NxpmError("invalid-input", "NXPM can import an AppImage or a Debian package.")

    def apply(self, package_id: str, operation: str, progress) -> None:
        key = _key(package_id)
        record = load_catalog().get("apps", {}).get(key)
        if not isinstance(record, dict):
            raise NxpmError("not-found", "That application is not in the Applications folder.")
        if operation == "remove":
            self._remove(key, record)
            progress(1.0, "Removed from Applications")
            return
        if operation == "update":
            information = str(record.get("update_info") or "")
            target = self._stored_file(key, record)
            progress(0.1, "Reading the embedded update channel")
            message = self._updater(target, information)
            progress(1.0, message)
            return
        raise NxpmError("unsupported", "Import an AppImage or a Debian package into Applications.")

    def _import_appimage(self, path: str, progress) -> Package:
        source = open_user_file(path)
        with source.open("rb") as handle:
            if not is_elf(handle.read(4)):
                raise NxpmError("invalid-input", "That file is not an AppImage.")
        key = _unique_key(safe_key(source.stem))
        progress(0.2, "Copying the AppImage into Applications")
        destination = applications_dir() / f"{key}.AppImage"
        shutil.copyfile(source, destination)
        destination.chmod(0o755)
        information = update_information(destination)
        name = _embedded_name(destination) or key
        summary = "Imported AppImage."
        if information and not describe_channel(information):
            summary = "Imported AppImage. Its update information is not a channel NXPM applies."
        write_desktop(key, name, destination, summary)
        record = {
            "name": name,
            "format": "appimage",
            "file": destination.name,
            "update_info": information,
            "summary": summary,
            "origin": "appimage",
        }
        update_catalog(lambda data: data["apps"].__setitem__(key, record))
        progress(1.0, "Added to Applications")
        return self._package(key, record)

    def _import_deb(self, path: str, progress) -> Package:
        source = open_user_file(path)
        if source.stat().st_size > _DEB_LIMIT:
            raise NxpmError("invalid-input", "The Debian package is larger than NXPM will unpack.")
        key = _unique_key(safe_key(source.stem))
        folder = applications_dir() / f"{key}.AppDir"
        progress(0.2, "Unpacking the Debian package without running its scripts")
        try:
            visible, _binary = extract_deb(source, folder)
        except Exception:
            if folder.exists():
                remove_tree(folder, applications_dir())
            raise
        launcher = folder / "AppRun"
        summary = "Imported from a Debian package. Maintainer scripts were not run."
        packed = _try_pack(folder, applications_dir() / f"{key}.AppImage")
        if packed is not None:
            remove_tree(folder, applications_dir())
            stored = packed
            file_format = "appimage"
            summary += " Packed as an AppImage."
        else:
            stored = launcher
            file_format = "appdir"
            summary += " Kept as an AppDir because appimagetool is not installed."
        write_desktop(key, visible, stored, summary)
        record = {
            "name": visible,
            "format": file_format,
            "file": stored.name if file_format == "appimage" else f"{key}.AppDir",
            "update_info": "",
            "summary": summary,
            "origin": "deb",
        }
        update_catalog(lambda data: data["apps"].__setitem__(key, record))
        progress(1.0, "Added to Applications")
        return self._package(key, record)

    def _remove(self, key: str, record: dict) -> None:
        root = applications_dir()
        file_format = record.get("format")
        if file_format == "appdir":
            remove_tree(root / f"{key}.AppDir", root)
        else:
            path = root / str(record.get("file") or f"{key}.AppImage")
            if path.exists() or path.is_symlink():
                remove_tree(path, root)
            partial = Path(str(path) + ".partial")
            if partial.exists():
                remove_tree(partial, root)
        menu = desktop_path(key)
        if menu.is_file():
            menu.unlink()
        update_catalog(lambda data: data["apps"].pop(key, None))

    def _stored_file(self, key: str, record: dict) -> Path:
        root = applications_dir()
        if record.get("format") == "appdir":
            raise NxpmError("unsupported", "An AppDir imported from a Debian package has no upstream update channel.")
        path = confine(root / str(record.get("file") or ""), root)
        if not path.is_file():
            raise NxpmError("not-found", "The AppImage file is no longer in Applications.")
        return path

    def _package(self, key: str, record: dict) -> Package:
        return Package(
            id=f"appimage:{key}",
            name=str(record.get("name") or key),
            source="appimage",
            summary=str(record.get("summary") or ""),
            version=str(record.get("version") or ""),
            kind="app",
            installed=True,
            update=bool(describe_channel(str(record.get("update_info") or ""))),
            risk="medium" if record.get("origin") == "deb" else "low",
        )


def _key(package_id: str) -> str:
    from nxpm.model import parse_package_id

    source, payload = parse_package_id(package_id)
    if source != "appimage":
        raise NxpmError("invalid-input", "That identifier is not an imported application.")
    return payload


def _unique_key(key: str) -> str:
    apps = load_catalog().get("apps", {})
    if key not in apps:
        return key
    raise NxpmError("failed", "An application with that name is already in Applications.")


def _try_pack(folder: Path, destination: Path) -> Path | None:
    tool = shutil.which("appimagetool")
    if tool is None:
        return None
    completed = subprocess.run(
        [tool, str(folder), str(destination)],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if completed.returncode != 0 or not destination.is_file():
        if destination.exists():
            destination.unlink()
        return None
    destination.chmod(0o755)
    return destination


def _embedded_name(path: Path) -> str:
    with path.open("rb") as handle:
        header = handle.read(4 * 1024 * 1024)
    offset = header.find(b"hsqs")
    if offset < 0:
        return ""
    tool = shutil.which("unsquashfs")
    if tool is None:
        return ""
    import tempfile

    with tempfile.TemporaryDirectory(prefix="nxpm-") as temporary:
        completed = subprocess.run(
            [tool, "-offset", str(offset), "-dest", temporary, "-no-progress", str(path), "usr/share/applications"],
            check=False,
            capture_output=True,
            text=True,
            timeout=20,
        )
        if completed.returncode != 0:
            return ""
        for desktop in Path(temporary).rglob("*.desktop"):
            for line in desktop.read_text(encoding="utf-8", errors="replace").splitlines():
                if line.startswith("Name="):
                    return " ".join(line.split("=", 1)[1].split())[:80]
    return ""
