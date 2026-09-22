#!/usr/bin/env python3
"""Create an Archiso profile from verified Nodalix release packages."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def prepare(assets, work):
    if work.exists():
        raise ValueError('Choose a new work directory; existing ISO builds are never erased')
    version = (ROOT / 'VERSION').read_text().strip()
    manifest = json.loads((assets / 'nodalix-manifest.json').read_text())
    if manifest['version'] != version or manifest['channel'] != 'stable':
        raise ValueError('ISO packages must match the stable source version')
    packages = []
    for entry in manifest['components']:
        name = entry['asset']
        if Path(name).name != name or not name.startswith('nodalix-'):
            raise ValueError('Unsafe package filename')
        package = assets / name
        if package.stat().st_size != entry['size'] or hashlib.sha256(package.read_bytes()).hexdigest() != entry['sha256']:
            raise ValueError(f'Invalid checksum: {name}')
        packages.append(package)
    profile = work / 'profile'
    shutil.copytree('/usr/share/archiso/configs/releng', profile)
    root = profile / 'airootfs'
    payload = root / 'usr/share/nodalix-installer'
    shutil.copytree(ROOT / 'iso/installer', payload)
    shutil.copytree(ROOT / 'iso/overlay', payload / 'overlay')
    shutil.copytree(ROOT / 'iso/overlay', root, dirs_exist_ok=True)
    package_dir = payload / 'packages'
    package_dir.mkdir()
    for package in packages:
        shutil.copy2(package, package_dir / package.name)
    shutil.copy2(assets / 'nodalix-manifest.json', package_dir)
    subprocess.run(['repo-add', str(package_dir / 'nodalix.db.tar.gz'),
                    *map(str, package_dir.glob('*.pkg.tar.zst'))], check=True)
    with (profile / 'pacman.conf').open('a') as stream:
        stream.write(f'\n[nodalix]\nSigLevel = Never\nServer = file://{package_dir}\n')
    # This repository is local to the build. The target gets verified package
    # files and retains normal signature validation for every Arch repository.
    base = 'base linux linux-firmware amd-ucode intel-ucode arch-install-scripts archinstall mkinitcpio mkinitcpio-archiso mkinitcpio-nfs-utils syslinux edk2-shell memtest86+ memtest86+-efi dosfstools e2fsprogs btrfs-progs xfsprogs cryptsetup lvm2 parted gptfdisk efibootmgr grub nano zsh grml-zsh-config openssh pciutils usbutils iw iwd wireless-regdb wget curl git rsync squashfs-tools less man-db dialog alsa-utils'.split()
    desktop = (ROOT / 'iso/installer/packages.txt').read_text().split()
    (profile / 'packages.x86_64').write_text('\n'.join(sorted(set(base + desktop + [e['package'] for e in manifest['components']]))) + '\n')
    definition = (profile / 'profiledef.sh').read_text()
    definition += f'''\niso_name="nodalix"
iso_label="NODALIX_020"
iso_publisher="Nodalix OS"
iso_application="Nodalix OS Live and Installer"
iso_version="{version}"
file_permissions+=(
 ["/usr/local/bin/nodalix-session"]="0:0:755"
 ["/usr/local/bin/nodalix-install"]="0:0:755"
 ["/usr/share/nodalix-installer/overlay/usr/local/bin/nodalix-session"]="0:0:755"
 ["/etc/sudoers.d/nodalix-live"]="0:0:440"
)
'''
    (profile / 'profiledef.sh').write_text(definition)
    shutil.copy2(ROOT / 'iso/installer/nodalix-install', root / 'usr/local/bin/nodalix-install')
    # Remove remote automation and SSH access inherited from Arch's rescue ISO.
    for directory in ('etc/systemd/system',):
        for path in (root / directory).rglob('*'):
            if path.is_symlink() and any(s in path.name for s in ('sshd', 'cloud-', 'networkd', 'iwd', 'ModemManager')):
                path.unlink()
    (root / 'root/.automated_script.sh').unlink(missing_ok=True)
    (root / 'root/.zlogin').write_text('')
    for path in (root / 'etc/systemd/system/getty@tty1.service.d').glob('*'):
        path.unlink()
    # Serial root console is retained solely as normal live rescue access.
    # It is not copied by the installation profile.
    custom = root / 'root/customize_airootfs.sh'
    custom.write_text('''#!/bin/bash
set -euo pipefail
useradd -m -G wheel,audio,video -s /bin/bash nodalix
passwd -d nodalix
install -d -o greeter -g greeter /var/lib/nodalix-greeter /var/lib/nodalix-greeter/cache
systemctl enable NetworkManager bluetooth greetd power-profiles-daemon
systemctl disable systemd-networkd systemd-networkd-wait-online iwd sshd 2>/dev/null || true
printf 'en_US.UTF-8 UTF-8\\nes_ES.UTF-8 UTF-8\\n' > /etc/locale.gen
locale-gen
printf 'LANG=en_US.UTF-8\\n' > /etc/locale.conf
''')
    sudoers = root / 'etc/sudoers.d/nodalix-live'
    sudoers.parent.mkdir(exist_ok=True)
    sudoers.write_text('nodalix ALL=(root) NOPASSWD: /usr/local/bin/nodalix-install\n')
    with (root / 'etc/greetd/config.toml').open('a') as stream:
        stream.write('\n[initial_session]\ncommand = "/usr/local/bin/nodalix-session"\nuser = "nodalix"\n')
    apps = root / 'usr/share/applications'
    apps.mkdir(parents=True, exist_ok=True)
    (apps / 'nodalix-install.desktop').write_text('[Desktop Entry]\nType=Application\nName=Install Nodalix\nName[es]=Instalar Nodalix\nExec=foot --title=Install-Nodalix sudo /usr/local/bin/nodalix-install\nIcon=system-software-install\nCategories=System;\n')
    for directory in ('syslinux', 'efiboot', 'grub'):
        for path in (profile / directory).rglob('*'):
            if path.is_file() and path.suffix in {'.cfg', '.conf'}:
                path.write_text(path.read_text().replace('Arch Linux', 'Nodalix OS 0.2.0'))
    return profile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    profile = prepare(args.assets.resolve(), args.work.resolve())
    if not args.prepare_only:
        subprocess.run(['mkarchiso', '-v', '-w', str(args.work.resolve() / 'build'),
                        '-o', str(args.output.resolve()), str(profile)], check=True)
        for iso in args.output.glob('nodalix-*.iso'):
            with iso.open('rb') as stream:
                digest = hashlib.file_digest(stream, 'sha256').hexdigest()
            iso.with_suffix('.iso.sha256').write_text(f'{digest}  {iso.name}\n')


if __name__ == '__main__':
    main()
