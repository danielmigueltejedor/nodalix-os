#!/usr/bin/env python3
"""Build the installable Nodalix OS ISO from verified release packages."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def iso_label(version: str) -> str:
    # 0.2.1 -> NODALIX_021
    token = version.replace(".", "").replace("-", "_").upper()
    token = re.sub(r"[^A-Z0-9_]", "_", token)
    return f"NODALIX_{token}"[:32]


def prepare(assets: Path, work: Path) -> tuple[Path, str]:
    if work.exists():
        raise ValueError(
            "Choose a new work directory; existing ISO builds are never erased"
        )

    version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()

    manifest_path = assets / "nodalix-manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

    if manifest["version"] != version:
        raise ValueError(
            f"Manifest version {manifest['version']} does not match VERSION {version}"
        )

    if manifest["channel"] != "stable":
        raise ValueError("Installable ISO images are built only from stable releases")

    packages: list[Path] = []

    for entry in manifest["components"]:
        if not entry.get("required", False):
            continue
        name = entry["asset"]

        if Path(name).name != name or not name.startswith("nodalix-"):
            raise ValueError(f"Unsafe package filename: {name}")

        package = assets / name

        if not package.is_file():
            raise ValueError(f"Missing release package: {name}")

        if package.stat().st_size != entry["size"]:
            raise ValueError(f"Invalid package size: {name}")

        if sha256(package) != entry["sha256"]:
            raise ValueError(f"Invalid package checksum: {name}")

        packages.append(package)

    profile = work / "profile"

    shutil.copytree(
        "/usr/share/archiso/configs/releng",
        profile,
        symlinks=True,
    )

    root = profile / "airootfs"

    payload = root / "usr/share/nodalix-installer"

    shutil.copytree(
        ROOT / "iso/installer",
        payload,
        ignore=shutil.ignore_patterns("__pycache__"),
    )

    shutil.copytree(
        ROOT / "iso/overlay",
        payload / "overlay",
    )

    shutil.copytree(
        ROOT / "iso/overlay",
        root,
        dirs_exist_ok=True,
    )

    # The live desktop only needs its default wallpaper. The complete
    # wallpaper package remains in the offline installer payload and is
    # installed into the final system.
    live_wallpaper = (
        ROOT
        / "packaging/nodalix-wallpapers"
        / "nodalix-alpine-mirror.jpg"
    )
    live_wallpaper_target = (
        root
        / "usr/share/backgrounds/nodalix/static"
        / "nodalix-alpine-mirror.jpg"
    )
    live_wallpaper_target.parent.mkdir(
        parents=True,
        exist_ok=True,
    )
    shutil.copy2(
        live_wallpaper,
        live_wallpaper_target,
    )

    # The installer and Archinstall profile use this instead of hardcoded
    # version numbers.
    shutil.copy2(
        ROOT / "VERSION",
        payload / "VERSION",
    )

    package_dir = payload / "packages"
    package_dir.mkdir()

    for package in packages:
        shutil.copy2(
            package,
            package_dir / package.name,
        )

    shutil.copy2(
        manifest_path,
        package_dir / "nodalix-manifest.json",
    )

    subprocess.run(
        [
            "repo-add",
            str(package_dir / "nodalix.db.tar.gz"),
            *map(str, sorted(package_dir.glob("*.pkg.tar.zst"))),
        ],
        check=True,
    )

    with (profile / "pacman.conf").open("a", encoding="utf-8") as stream:
        stream.write(
            "\n"
            "[nodalix]\n"
            "SigLevel = Never\n"
            f"Server = file://{package_dir}\n"
        )

    base = (
        "base linux linux-firmware amd-ucode intel-ucode "
        "arch-install-scripts archinstall "
        "mkinitcpio mkinitcpio-archiso mkinitcpio-nfs-utils "
        "nbd syslinux edk2-shell memtest86+ memtest86+-efi "
        "dosfstools e2fsprogs btrfs-progs xfsprogs "
        "cryptsetup lvm2 parted gptfdisk efibootmgr grub "
        "nano zsh grml-zsh-config openssh pciutils usbutils "
        "iw iwd wireless-regdb wget curl git rsync "
        "squashfs-tools less man-db dialog alsa-utils"
    ).split()

    desktop = (ROOT / "iso/installer/packages.txt").read_text(
        encoding="utf-8"
    ).split()

    # Keep the live medium below GitHub's release-asset limits without
    # trimming hardware firmware.
    desktop = [
        package
        for package in desktop
        if package not in {
            "noto-fonts",
            "noto-fonts-cjk",
            "noto-fonts-emoji",
            "firefox",
            "gcc",
        }
    ]

    base += [
        "python-systemd",
        "timeshift",
    ]

    # Only install components required by the live desktop itself.
    #
    # Every release package is still stored under
    # /usr/share/nodalix-installer/packages and is installed into the
    # target system by the installer. Installing all of them into the
    # live root as well duplicates large assets such as wallpapers and
    # emoji fonts and can push the ISO over GitHub Releases 2 GiB limit.
    live_nodalix_packages = [
        "nodalix-gnome",
        "nodalix-settings",
        "nodalix-control-center",
        "nodalix-video-wallpapers",
        "nodalix-integrations",
    ]

    (profile / "packages.x86_64").write_text(
        "\n".join(
            sorted(
                set(
                    base
                    + desktop
                    + live_nodalix_packages
                )
            )
        )
        + "\n",
        encoding="utf-8",
    )

    definition_path = profile / "profiledef.sh"
    definition = definition_path.read_text(encoding="utf-8")

    definition += f"""
iso_name="nodalix"
iso_label="{iso_label(version)}"
iso_publisher="Nodalix OS"
iso_application="Nodalix OS Live and Installer"
iso_version="{version}"
file_permissions+=(
 ["/usr/local/bin/nodalix-session"]="0:0:755"
 ["/usr/local/bin/nodalix-install"]="0:0:755"
 ["/usr/share/nodalix-installer/overlay/usr/local/bin/nodalix-session"]="0:0:755"
 ["/etc/sudoers.d/nodalix-live"]="0:0:440"
)
"""

    definition_path.write_text(
        definition,
        encoding="utf-8",
    )

    shutil.copy2(
        ROOT / "iso/installer/nodalix-install",
        root / "usr/local/bin/nodalix-install",
    )

    # Remove remote automation and SSH access inherited from Arch's rescue ISO.
    for directory in ("etc/systemd/system",):
        for path in (root / directory).rglob("*"):
            if (
                path.is_symlink()
                and any(
                    token in path.name
                    for token in (
                        "sshd",
                        "cloud-",
                        "networkd",
                        "iwd",
                        "ModemManager",
                        "vbox",
                        "vmware",
                        "vmtools",
                        "livecd-talk",
                    )
                )
            ):
                path.unlink()

    (root / "root/.automated_script.sh").unlink(
        missing_ok=True
    )

    (root / "root/.zlogin").write_text(
        "",
        encoding="utf-8",
    )

    getty = root / "etc/systemd/system/getty@tty1.service.d"

    if getty.exists():
        for path in getty.glob("*"):
            path.unlink()

    custom = root / "root/customize_airootfs.sh"

    custom.write_text(
        """#!/bin/bash
set -euo pipefail

useradd -m -G wheel,audio,video -s /bin/bash nodalix
passwd -d nodalix


install -d /etc/dconf/db/local.d /etc/dconf/profile
printf 'user-db:user\\n system-db:local\\n' | sed 's/^ //' > /etc/dconf/profile/user
printf '[org/gnome/desktop/session]\\nidle-delay=uint32 0\\n[org/gnome/desktop/screensaver]\\nlock-enabled=false\\n' > /etc/dconf/db/local.d/00-live
dconf update
chown -R nodalix:nodalix /home/nodalix

# Keep the live medium lean without removing anything from the installed
# system. These heavy integrations are not needed by the installer session;
# the wallpaper-engine package itself remains embedded in the installer
# payload for deployment to the target system.
for pkg in nodalix-wallpaper-engine rclone; do
    if pacman -Q "$pkg" >/dev/null 2>&1; then
        pacman -Qlq "$pkg" \
            | while IFS= read -r path; do
                if [[ -f "$path" || -L "$path" ]]; then
                    rm -f -- "$path"
                fi
              done
    fi
done

# Documentation and package caches are unnecessary on the ephemeral live
# session and only increase the release asset size.
rm -rf \
    /usr/share/doc/* \
    /usr/share/man/* \
    /usr/share/info/* \
    /var/cache/pacman/pkg/*

systemctl enable \
    NetworkManager \
    bluetooth \
    gdm \
    power-profiles-daemon

systemctl disable \
    systemd-networkd \
    systemd-networkd-wait-online \
    iwd \
    sshd \
    nodalix-icloud-drive.service \
    nodalix-icloud-sync.timer 2>/dev/null || true

printf 'en_US.UTF-8 UTF-8\\nes_ES.UTF-8 UTF-8\\n' > /etc/locale.gen
locale-gen
printf 'LANG=en_US.UTF-8\\n' > /etc/locale.conf
""",
        encoding="utf-8",
    )

    sudoers = root / "etc/sudoers.d/nodalix-live"
    sudoers.parent.mkdir(
        parents=True,
        exist_ok=True,
    )

    sudoers.write_text(
        "nodalix ALL=(root) NOPASSWD: /usr/local/bin/nodalix-install\n",
        encoding="utf-8",
    )

    gdm_config = root / "etc/gdm/custom.conf"
    gdm_config.parent.mkdir(parents=True, exist_ok=True)
    gdm_config.write_text("[daemon]\nWaylandEnable=true\nAutomaticLoginEnable=true\nAutomaticLogin=nodalix\n", encoding="utf-8")

    applications = root / "usr/share/applications"
    applications.mkdir(
        parents=True,
        exist_ok=True,
    )

    (applications / "nodalix-install.desktop").write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Install Nodalix\n"
        "Name[es]=Instalar Nodalix\n"
        "Exec=kgx --title=Install-Nodalix -- sudo /usr/local/bin/nodalix-install\n"
        "Icon=system-software-install\n"
        "Categories=System;\n",
        encoding="utf-8",
    )

    for directory in (
        "syslinux",
        "efiboot",
        "grub",
    ):
        target = profile / directory

        if not target.exists():
            continue

        for path in target.rglob("*"):
            if (
                path.is_file()
                and path.suffix in {".cfg", ".conf"}
            ):
                text = path.read_text(
                    encoding="utf-8"
                )

                path.write_text(
                    text.replace(
                        "Arch Linux",
                        f"Nodalix OS {version}",
                    ),
                    encoding="utf-8",
                )

    return profile, version


def main() -> None:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--assets",
        type=Path,
        required=True,
    )

    parser.add_argument(
        "--work",
        type=Path,
        required=True,
    )

    parser.add_argument(
        "--output",
        type=Path,
        required=True,
    )

    parser.add_argument(
        "--prepare-only",
        action="store_true",
    )

    args = parser.parse_args()

    assets = args.assets.resolve()
    work = args.work.resolve()
    output = args.output.resolve()

    output.mkdir(
        parents=True,
        exist_ok=True,
    )

    profile, version = prepare(
        assets,
        work,
    )

    if args.prepare_only:
        print(profile)
        return

    subprocess.run(
        [
            "mkarchiso",
            "-v",
            "-w",
            str(work / "build"),
            "-o",
            str(output),
            str(profile),
        ],
        check=True,
    )

    expected = output / f"nodalix-{version}-x86_64.iso"

    if not expected.is_file():
        images = sorted(
            output.glob("nodalix-*.iso")
        )

        raise RuntimeError(
            "Expected ISO "
            f"{expected.name}; generated: "
            + ", ".join(
                image.name
                for image in images
            )
        )

    checksum = sha256(expected)

    checksum_path = expected.with_suffix(
        ".iso.sha256"
    )

    checksum_path.write_text(
        f"{checksum}  {expected.name}\n",
        encoding="utf-8",
    )

    print(expected)
    print(checksum_path)


if __name__ == "__main__":
    main()
