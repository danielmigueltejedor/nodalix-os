"""Nodalix extension of Archinstall; disk handling stays in Archinstall."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from pathlib import Path

from archinstall.default_profiles.profile import (
    DisplayServerType,
    Profile,
    ProfileType,
)

PAYLOAD = Path(
    "/usr/share/nodalix-installer"
)


def expected_version(
    payload: Path = PAYLOAD,
) -> str:
    version_file = payload / "VERSION"

    if not version_file.is_file():
        raise ValueError(
            "The ISO contains no Nodalix VERSION file"
        )

    version = version_file.read_text(
        encoding="utf-8"
    ).strip()

    if not version:
        raise ValueError(
            "The ISO contains an empty Nodalix VERSION file"
        )

    return version


def sha256(path: Path) -> str:
    digest = hashlib.sha256()

    with path.open("rb") as stream:
        for chunk in iter(
            lambda: stream.read(1024 * 1024),
            b"",
        ):
            digest.update(chunk)

    return digest.hexdigest()


def verified_packages(
    payload: Path = PAYLOAD,
) -> list[Path]:
    version = expected_version(payload)

    manifest_path = (
        payload
        / "packages"
        / "nodalix-manifest.json"
    )

    manifest = json.loads(
        manifest_path.read_text(
            encoding="utf-8"
        )
    )

    if manifest["version"] != version:
        raise ValueError(
            f"ISO version {version} does not match "
            f"manifest version {manifest['version']}"
        )

    if manifest["channel"] != "stable":
        raise ValueError(
            "The installable ISO must contain a stable manifest"
        )

    packages: list[Path] = []

    for item in manifest["components"]:
        if not item.get("required", False):
            continue
        name = item["asset"]

        if (
            Path(name).name != name
            or not name.startswith("nodalix-")
        ):
            raise ValueError(
                f"Invalid package path: {name}"
            )

        path = (
            payload
            / "packages"
            / name
        )

        if not path.is_file():
            raise ValueError(
                f"ISO package is missing: {name}"
            )

        if path.stat().st_size != item["size"]:
            raise ValueError(
                f"Package size failed: {name}"
            )

        if sha256(path) != item["sha256"]:
            raise ValueError(
                f"Package checksum failed: {name}"
            )

        packages.append(path)

    if not packages:
        raise ValueError(
            "The ISO contains no Nodalix packages"
        )

    return packages


class NodalixProfile(Profile):
    def __init__(self):
        super().__init__(
            "Nodalix",
            ProfileType.DesktopEnv,
            support_gfx_driver=True,
            display_server=DisplayServerType.Wayland,
        )

    @property
    def packages(self):
        return (
            PAYLOAD
            / "packages.txt"
        ).read_text(
            encoding="utf-8"
        ).split()

    @property
    def services(self):
        return [
            "NetworkManager",
            "bluetooth",
            "gdm",
            "power-profiles-daemon",
        ]

    def post_install(
        self,
        install_session,
    ):
        target = (
            install_session
            .target
            .resolve()
        )

        if (
            target == Path("/")
            or not (
                target
                / "etc/arch-release"
            ).exists()
        ):
            raise RuntimeError(
                "Refusing to deploy outside "
                "the installed target"
            )

        packages = verified_packages()

        cache = (
            target
            / "var/cache/nodalix-installer"
        )

        cache.mkdir(
            parents=True,
            exist_ok=True,
        )

        for package in packages:
            shutil.copy2(
                package,
                cache / package.name,
            )

        subprocess.run(
            [
                "arch-chroot",
                str(target),
                "pacman",
                "-U",
                "--noconfirm",
                *[
                    "/var/cache/nodalix-installer/"
                    + package.name
                    for package in packages
                ],
            ],
            check=True,
        )

        shutil.copytree(
            PAYLOAD / "overlay",
            target,
            dirs_exist_ok=True,
        )

        subprocess.run(["arch-chroot", str(target), "nodalix-gnome-migrate", "--system"], check=True)

        install_session.enable_service(
            self.services
            + [
                "nodalix-update-check.timer",
            ]
        )

        shutil.copy2(
            PAYLOAD / "finish-hardware.py",
            cache / "finish-hardware.py",
        )

        subprocess.run(
            [
                "arch-chroot",
                str(target),
                "python3",
                "/var/cache/nodalix-installer/finish-hardware.py",
            ],
            check=True,
        )

        # The live account, autologin and sudo exception
        # never enter the installed target.
        shutil.rmtree(cache)

    def provision(
        self,
        install_session,
        users,
    ):
        for user in users:
            home = (
                install_session.target
                / "home"
                / user.username
            )

            skeleton = (
                PAYLOAD
                / "overlay/etc/skel"
            )

            for source in skeleton.rglob("*"):
                destination = (
                    home
                    / source.relative_to(
                        skeleton
                    )
                )

                if (
                    source.is_file()
                    and not destination.exists()
                ):
                    destination.parent.mkdir(
                        parents=True,
                        exist_ok=True,
                    )

                    shutil.copy2(
                        source,
                        destination,
                    )

            # GNOME has no legacy Hyprland skeleton. A new user's directories
            # must exist before ownership is assigned by arch-chroot.
            for directory in ('.config', '.local'):
                (home / directory).mkdir(parents=True, exist_ok=True)

            subprocess.run(
                [
                    "arch-chroot",
                    str(
                        install_session.target
                    ),
                    "chown",
                    "-R",
                    user.username
                    + ":"
                    + user.username,
                    "/home/"
                    + user.username
                    + "/.config",
                    "/home/"
                    + user.username
                    + "/.local",
                ],
                check=True,
            )
