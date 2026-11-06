"""Turn a downloaded Debian package into an application bundle.

The package is extracted as data. Maintainer scripts are never executed and
nothing is installed into the system package database. Files keep no setuid
bit. The result is an AppDir in the Applications folder, and an AppImage when
``appimagetool`` is available.
"""

from __future__ import annotations

import io
import re
import stat
import tarfile
from pathlib import Path

from nxpm.errors import NxpmError

_BINARY = re.compile(r"^usr/(?:bin|games)/[A-Za-z0-9._+-]+$")
_MEMBER = re.compile(r"^[A-Za-z0-9._+-]+$")


def extract_deb(source: Path, destination: Path) -> tuple[str, str]:
    """Extract ``data.tar`` into ``destination`` and return ``(name, binary)``."""

    members = _ar_members(source.read_bytes())
    if "debian-binary" not in members:
        raise NxpmError("invalid-input", "That file is not a Debian package.")
    data_name = next((name for name in members if name.startswith("data.tar")), "")
    if not data_name:
        raise NxpmError("invalid-input", "The Debian package has no data archive.")
    destination.mkdir(parents=True, exist_ok=True)
    try:
        with tarfile.open(fileobj=io.BytesIO(members[data_name]), mode="r:*") as archive:
            archive.extractall(destination, filter="data")
    except (tarfile.TarError, OSError) as exc:
        raise NxpmError("failed", "The Debian package data could not be unpacked.") from exc
    _neutralize(destination)
    desktop = _first_desktop(destination)
    name, executable = _launch_target(destination, desktop)
    binary = destination / executable
    if not binary.is_file():
        raise NxpmError("unsupported", "The Debian package does not contain a program NXPM can launch.")
    binary.chmod(0o755)
    _write_apprun(destination, executable)
    return name, executable


def _ar_members(blob: bytes) -> dict[str, bytes]:
    if not blob.startswith(b"!<arch>\n"):
        raise NxpmError("invalid-input", "That file is not a Debian package.")
    position = 8
    found: dict[str, bytes] = {}
    while position + 60 <= len(blob):
        header = blob[position : position + 60]
        if len(header) < 60 or header[58:60] != b"`\n":
            break
        raw_name = header[:16].decode("ascii", "replace").strip()
        try:
            size = int(header[48:58].decode("ascii").strip() or "0")
        except ValueError as exc:
            raise NxpmError("invalid-input", "The Debian archive header is not valid.") from exc
        position += 60
        if size < 0 or position + size > len(blob):
            raise NxpmError("invalid-input", "The Debian archive is truncated.")
        payload = blob[position : position + size]
        position += size + (size % 2)
        name = raw_name.rstrip("/")
        if not _MEMBER.match(name):
            continue
        found[name] = payload
    return found


def _neutralize(root: Path) -> None:
    for path in root.rglob("*"):
        if path.is_symlink():
            continue
        if path.is_dir():
            path.chmod(0o755)
        elif path.is_file():
            path.chmod(0o644)
            mode = path.stat().st_mode
            if mode & (stat.S_ISUID | stat.S_ISGID):
                path.chmod(stat.S_IMODE(mode) & ~(stat.S_ISUID | stat.S_ISGID))


def _first_desktop(root: Path) -> Path | None:
    directory = root / "usr" / "share" / "applications"
    if not directory.is_dir():
        return None
    files = sorted(path for path in directory.glob("*.desktop") if path.is_file())
    return files[0] if files else None


def _launch_target(root: Path, desktop: Path | None) -> tuple[str, str]:
    name = root.name
    command = ""
    if desktop is not None:
        for raw in desktop.read_text(encoding="utf-8", errors="replace").splitlines():
            if raw.startswith("Name=") and name == root.name:
                name = raw.split("=", 1)[1].strip() or name
            if raw.startswith("Exec=") and not command:
                command = raw.split("=", 1)[1].strip()
    token = ""
    for part in command.split():
        if part.startswith("%"):
            continue
        token = part.strip("\"'")
        break
    if token.startswith("/"):
        token = token[1:]
    if _BINARY.match(token) and (root / token).is_file():
        return _visible(name), token
    for directory in ("usr/bin", "usr/games"):
        folder = root / directory
        if not folder.is_dir():
            continue
        programs = sorted(path for path in folder.iterdir() if path.is_file() and not path.is_symlink())
        if programs:
            relative = programs[0].relative_to(root).as_posix()
            if _BINARY.match(relative):
                return _visible(name), relative
    raise NxpmError("unsupported", "The Debian package does not contain a program NXPM can launch.")


def _visible(name: str) -> str:
    cleaned = " ".join(name.replace("/", " ").split())
    return cleaned[:80] or "Application"


def _write_apprun(root: Path, relative: str) -> None:
    if not _BINARY.match(relative):
        raise NxpmError("invalid-input", "The program path inside the bundle is not valid.")
    script = "#!/bin/sh\nset -eu\nHERE=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n" f'exec "$HERE/{relative}" "$@"\n'
    path = root / "AppRun"
    path.write_text(script, encoding="utf-8")
    path.chmod(0o755)
