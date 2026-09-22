"""Nodalix extension of Archinstall; disk handling stays in Archinstall."""
import hashlib
import json
import shutil
import subprocess
from pathlib import Path
from archinstall.default_profiles.profile import Profile, ProfileType, DisplayServerType

PAYLOAD = Path('/usr/share/nodalix-installer')


def verified_packages(payload=PAYLOAD):
    manifest = json.loads((payload / 'packages/nodalix-manifest.json').read_text())
    if manifest['version'] != '0.2.0' or manifest['channel'] != 'stable':
        raise ValueError('The ISO must contain the stable 0.2.0 manifest')
    packages = []
    for item in manifest['components']:
        name = item['asset']
        if Path(name).name != name or not name.startswith('nodalix-'):
            raise ValueError('Invalid package path')
        path = payload / 'packages' / name
        if path.stat().st_size != item['size'] or hashlib.sha256(path.read_bytes()).hexdigest() != item['sha256']:
            raise ValueError(f'Package checksum failed: {name}')
        packages.append(path)
    if not packages:
        raise ValueError('The ISO contains no Nodalix packages')
    return packages


class NodalixProfile(Profile):
    def __init__(self):
        super().__init__('Nodalix', ProfileType.DesktopEnv,
                         support_gfx_driver=True, display_server=DisplayServerType.Wayland)

    @property
    def packages(self):
        return (PAYLOAD / 'packages.txt').read_text().split()

    @property
    def services(self):
        return ['NetworkManager', 'bluetooth', 'greetd', 'power-profiles-daemon']

    def post_install(self, install_session):
        target = install_session.target.resolve()
        if target == Path('/') or not (target / 'etc/arch-release').exists():
            raise RuntimeError('Refusing to deploy outside the installed target')
        packages = verified_packages()
        cache = target / 'var/cache/nodalix-installer'
        cache.mkdir(parents=True, exist_ok=True)
        for package in packages:
            shutil.copy2(package, cache / package.name)
        subprocess.run(['arch-chroot', str(target), 'pacman', '-U', '--noconfirm',
                        *['/var/cache/nodalix-installer/' + p.name for p in packages]], check=True)
        shutil.copytree(PAYLOAD / 'overlay', target, dirs_exist_ok=True)
        subprocess.run(['arch-chroot', str(target), 'install', '-d', '-o', 'greeter', '-g', 'greeter',
                        '/var/lib/nodalix-greeter', '/var/lib/nodalix-greeter/cache'], check=True)
        install_session.enable_service(self.services + ['nodalix-update-check.timer'])
        shutil.copy2(PAYLOAD / 'finish-hardware.py', cache / 'finish-hardware.py')
        subprocess.run(['arch-chroot', str(target), 'python3', '/var/cache/nodalix-installer/finish-hardware.py'], check=True)
        # The live account, autologin and sudo exception never enter the target.
        shutil.rmtree(cache)
