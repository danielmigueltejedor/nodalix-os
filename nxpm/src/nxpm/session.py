"""Install a user D-Bus service and a read-only update timer."""

from __future__ import annotations

import os
import sys
from pathlib import Path

from nxpm.paths import state_dir


def install_session() -> None:
    executable = f"{sys.executable} -m nxpm"
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
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
