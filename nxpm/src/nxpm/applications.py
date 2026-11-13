"""User applications live in ~/Applications, the way macOS uses /Applications.

The base operating system is not converted. Pacman keeps the kernel, the
drivers and the libraries that the machine boots with. NXPM only places
applications that the user imports into this folder.
"""

from __future__ import annotations

import fcntl
import json
import os
import re
from pathlib import Path
from nxpm.errors import NxpmError

_KEY = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,80}$")
_MAX_FILE = 8 * 1024 * 1024 * 1024


def applications_dir() -> Path:
    override = os.environ.get("NXPM_APPLICATIONS_DIR")
    path = Path(override).expanduser() if override else Path.home() / "Applications"
    path.mkdir(parents=True, exist_ok=True)
    return path.resolve()


def data_home() -> Path:
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))


def catalog_path() -> Path:
    from nxpm.paths import state_dir

    return state_dir() / "applications.json"


def safe_key(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9._+-]+", "-", (value or "").strip()).strip(".-")
    if not _KEY.match(cleaned):
        raise NxpmError("invalid-input", "The application name cannot be used as a file name.")
    return cleaned


def confine(path: Path, root: Path) -> Path:
    """Return a resolved path that is inside ``root``."""

    resolved_root = root.resolve()
    resolved = path.resolve()
    if resolved != resolved_root and resolved_root not in resolved.parents:
        raise NxpmError("invalid-input", "NXPM refused a path outside the Applications folder.")
    return resolved


def open_user_file(text: str) -> Path:
    if not text or "\x00" in text:
        raise NxpmError("invalid-input", "The file path is not valid.")
    candidate = Path(text).expanduser()
    try:
        resolved = candidate.resolve(strict=True)
    except OSError as exc:
        raise NxpmError("not-found", "That file cannot be opened.") from exc
    if not resolved.is_file():
        raise NxpmError("invalid-input", "NXPM can import a regular file.")
    try:
        size = resolved.stat().st_size
    except OSError as exc:
        raise NxpmError("not-found", "That file cannot be opened.") from exc
    if size <= 0 or size > _MAX_FILE:
        raise NxpmError("invalid-input", "The file is empty or larger than NXPM will import.")
    return resolved


def load_catalog() -> dict:
    path = catalog_path()
    if not path.is_file():
        return {"apps": {}}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"apps": {}}
    if not isinstance(data, dict) or not isinstance(data.get("apps"), dict):
        return {"apps": {}}
    return data


def update_catalog(mutator) -> dict:
    path = catalog_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    lock_path = path.with_name(path.name + ".lock")
    with lock_path.open("a+", encoding="utf-8") as lock_handle:
        fcntl.flock(lock_handle.fileno(), fcntl.LOCK_EX)
        try:
            if path.is_file():
                try:
                    data = json.loads(path.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    data = {"apps": {}}
            else:
                data = {"apps": {}}
            if not isinstance(data, dict):
                data = {"apps": {}}
            if not isinstance(data.get("apps"), dict):
                data["apps"] = {}
            mutator(data)
            serialized = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
            temporary = path.with_name(path.name + ".tmp")
            with temporary.open("w", encoding="utf-8") as handle:
                handle.write(serialized)
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, path)
        finally:
            fcntl.flock(lock_handle.fileno(), fcntl.LOCK_UN)
    return data


def desktop_path(key: str) -> Path:
    if not _KEY.match(key):
        raise NxpmError("invalid-input", "The application name is not valid.")
    directory = data_home() / "applications"
    directory.mkdir(parents=True, exist_ok=True)
    return directory / f"os.nodalix.nxpm.{key}.desktop"


def desktop_exec_escape(value: str) -> str:
    """Quote an Exec path for a freedesktop desktop entry.

    Absolute application paths are always double-quoted after escaping the
    reserved characters named by the desktop-entry spec (including ``'`` and
    ``%`` → ``%%``).
    """
    text = str(value)
    escaped = (
        text.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("$", "\\$")
        .replace("`", "\\`")
        .replace("%", "%%")
    )
    return f'"{escaped}"'


_ICON = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,120}$")


def write_desktop(key: str, name: str, executable: Path, comment: str, icon: str = "") -> Path:
    target = confine(executable, applications_dir())
    icon_name = icon if _ICON.match(icon or "") else "application-x-executable"
    visible = " ".join((name or key).split()) or key
    comment_text = " ".join((comment or "").split())
    exec_value = desktop_exec_escape(str(target))
    content = (
        "[Desktop Entry]\n"
        "Type=Application\n"
        f"Name={visible}\n"
        f"Exec={exec_value}\n"
        f"Icon={icon_name}\n"
        "Terminal=false\n"
        "Categories=Utility;\n"
        f"Comment={comment_text}\n"
        "X-Nodalix-Package-Manager=nxpm\n"
    )
    path = desktop_path(key)
    path.write_text(content, encoding="utf-8")
    return path


def remove_tree(path: Path, root: Path) -> None:
    root_resolved = root.resolve()
    if path.is_symlink():
        parent = path.parent.resolve()
        if parent != root_resolved and root_resolved not in parent.parents:
            raise NxpmError("invalid-input", "NXPM refused a path outside the Applications folder.")
        path.unlink()
        return
    target = confine(path, root)
    if target == root_resolved:
        raise NxpmError("invalid-input", "NXPM will not delete the Applications folder.")
    if target.is_file():
        target.unlink()
        return
    if not target.is_dir():
        return
    for child in sorted(target.rglob("*"), reverse=True):
        if child.is_symlink() or child.is_file():
            child.unlink()
        elif child.is_dir():
            confine(child, root).rmdir()
    target.rmdir()
