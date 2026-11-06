"""Install a user D-Bus service and a read-only update timer."""

from __future__ import annotations

import os
import shlex
import sys
from pathlib import Path

from nxpm.paths import state_dir


PACKAGED_ROOT = Path("/usr/lib/nodalix-nxpm")


GENERATED_FILES = (
    ("dbus-1/services", "os.nodalix.Nxpm.service", "data"),
    ("systemd/user", "nxpm.service", "config"),
    ("systemd/user", "nxpm-check.service", "config"),
    ("systemd/user", "nxpm-check.timer", "config"),
)


def _user_dirs() -> tuple[Path, Path]:
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    return data, config


def _remove_generated_overrides() -> list[Path]:
    """Delete user files an earlier install_session() wrote, and nothing else.

    A generated file is recognised by its interpreter command (``-m nxpm``); a
    file the user wrote or edited without it is left alone.
    """
    data, config = _user_dirs()
    roots = {"data": data, "config": config}
    removed = []
    for directory, name, root in GENERATED_FILES:
        path = roots[root] / directory / name
        try:
            if path.is_file() and " -m nxpm" in path.read_text(encoding="utf-8"):
                path.unlink()
                removed.append(path)
        except (OSError, UnicodeDecodeError):
            continue
    return removed


def install_session() -> None:
    """Install the per-user services, or defer to the system package."""
    if (PACKAGED_ROOT / "lib").is_dir():
        # The package already installs the D-Bus service and the check units;
        # older per-user copies would shadow them with a stale interpreter.
        for path in _remove_generated_overrides():
            print(f"Removed outdated user override {path}")
        print("NXPM is installed system-wide; its user services are already in place.")
        print("Enable the check with: systemctl --user enable --now nxpm-check.timer")
        return
    executable = shlex.join([sys.executable, "-m", "nxpm"])
    data, config = _user_dirs()
    dbus_dir = data / "dbus-1" / "services"
    systemd_dir = config / "systemd" / "user"
    dbus_dir.mkdir(parents=True, exist_ok=True)
    systemd_dir.mkdir(parents=True, exist_ok=True)
    state_dir()

    (dbus_dir / "os.nodalix.Nxpm.service").write_text(
        "\n".join(
            [
                "[D-BUS Service]",
                "Name=os.nodalix.Nxpm",
                f"Exec={executable} daemon",
                "SystemdService=nxpm.service",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (systemd_dir / "nxpm.service").write_text(
        "\n".join(
            [
                "[Unit]",
                "Description=Nodalix Package Manager",
                "PartOf=graphical-session.target",
                "",
                "[Service]",
                "Type=dbus",
                "BusName=os.nodalix.Nxpm",
                f"ExecStart={executable} daemon",
                "",
                "[Install]",
                "WantedBy=default.target",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (systemd_dir / "nxpm-check.service").write_text(
        "\n".join(
            [
                "[Unit]",
                "Description=Check for Nodalix software updates",
                "",
                "[Service]",
                "Type=oneshot",
                f"ExecStart={executable} check",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (systemd_dir / "nxpm-check.timer").write_text(
        "\n".join(
            [
                "[Unit]",
                "Description=Periodic read-only software update check",
                "",
                "[Timer]",
                "OnBootSec=5min",
                "OnUnitActiveSec=6h",
                "Persistent=true",
                "",
                "[Install]",
                "WantedBy=timers.target",
                "",
            ]
        ),
        encoding="utf-8",
    )
    policy = Path(__file__).resolve().parents[2] / "share" / "os.nodalix.nxpm.policy"
    print(f"User service files installed. Polkit policy is at {policy}.")
    print("Enable the check with: systemctl --user enable --now nxpm-check.timer")
