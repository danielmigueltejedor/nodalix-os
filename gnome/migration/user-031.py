#!/usr/bin/python3
"""Adopt packaged desktop helpers once, preserving per-user preferences."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


UNITS = ('steam-auto-gamemode.service', 'steam-desktop-sync.service',
         'steam-desktop-sync.timer', 'nodalix-craft-gnome.service')


# SHA-256 of the original prototypes, with the home path normalised to %h.
PROTOTYPES = {
    "steam-auto-gamemode.service": "39399a6ed53e6328464ebd0fa5a20d54e850be53a01002974e05eda374ad3ea8",
    "steam-desktop-sync.service": "b7a23c3787ee385e706f00f6d09a49d095988e651e4b0f6a718a14e2ae37a088",
    "steam-desktop-sync.timer": "141afd689c664a3049633222a1bf27eaa7f3f83b4f32225e40904957f127a4c1",
    "nodalix-craft-gnome.service": "035ccb0e28404bebcd07395f31c122357262960293b6c4314bf92d9d712fd66e"
}


def retire_local_units(config, backup):
    """Only retire the known local prototypes that shadow packaged units."""
    retired = []
    for name in UNITS:
        source = config / 'systemd/user' / name
        target = backup / 'units' / name
        if source.is_symlink():
            continue
        if source.is_file():
            text = source.read_text()
            digest = hashlib.sha256(text.replace(str(Path.home()), '%h').encode()).hexdigest()
            if digest != PROTOTYPES[name]:
                continue
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            if target.exists():
                if target.read_bytes() != source.read_bytes():
                    raise RuntimeError('Local unit changed since backup: ' + name)
                source.unlink()
            else:
                shutil.move(str(source), str(target))
        elif source.exists() or not target.is_file():
            continue
        # Old `enable` links point at the now-retired per-user unit. Remove
        # only those links, allowing the packaged graphical-session wants in.
        for link in (config / 'systemd/user').glob('*.wants/' + name):
            if not link.is_symlink():
                continue
            destination = Path(os.path.abspath(link.parent / link.readlink()))
            if destination != source:
                continue
            saved = backup / 'wants' / link.parent.name / name
            saved.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            if saved.is_symlink() and saved.readlink() == link.readlink():
                link.unlink()
            elif not saved.exists() and not saved.is_symlink():
                shutil.move(str(link), str(saved))
            else:
                raise RuntimeError('Conflicting service link backup: ' + name)
        retired.append(name)
    return retired


def enable_startup_cursor(settings):
    uuid = 'nodalix-startup-cursor@local'
    enabled = list(settings.get_strv('enabled-extensions'))
    if uuid not in enabled and uuid not in settings.get_strv('disabled-extensions'):
        settings.set_strv('enabled-extensions', enabled + [uuid])


def retire_extension_prototypes(data, backup, prototypes):
    """Archive exact previous snapshots; retain custom or third-party copies."""
    for row in prototypes:
        folder = data / 'gnome-shell/extensions' / row['uuid']
        if not folder.is_dir() or folder.is_symlink():
            continue
        files = {str(p.relative_to(folder)) for p in folder.rglob('*')
                 if p.is_file() and p.name != 'gschemas.compiled'}
        if files != set(row['files']):
            continue
        if any(hashlib.sha256((folder / name).read_bytes()).hexdigest() != digest
               for name, digest in row['files'].items()):
            continue
        target = backup / 'extensions' / row['uuid']
        if target.exists():
            raise RuntimeError('Extension backup already exists: ' + row['uuid'])
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        shutil.move(str(folder), str(target))


def main():
    if 'gnome' not in os.environ.get('XDG_CURRENT_DESKTOP', '').lower().split(':'):
        return
    from gi.repository import Gio
    home = Path.home()
    state = Path(os.environ.get('XDG_STATE_HOME', home / '.local/state'))
    config = Path(os.environ.get('XDG_CONFIG_HOME', home / '.config'))
    data = Path(os.environ.get('XDG_DATA_HOME', home / '.local/share'))
    root = state / 'nodalix/migrations'
    marker = root / 'gnome-0.3.1.done'
    if marker.is_file():
        return
    backup = root / 'gnome-0.3.1'
    backup.mkdir(parents=True, exist_ok=True, mode=0o700)
    settings = Gio.Settings.new('org.gnome.shell')
    saved = backup / 'extensions.json'
    if not saved.exists():
        saved.write_text(json.dumps({key: list(settings.get_strv(key)) for key in
                                    ('enabled-extensions', 'disabled-extensions')}, indent=2) + '\n')
    retire_local_units(config, backup)
    prototypes = Path('/usr/share/nodalix/gnome-extension-prototypes.json')
    if prototypes.is_file():
        retire_extension_prototypes(data, backup, json.loads(prototypes.read_text()))
    subprocess.run(['systemctl', '--user', 'daemon-reload'], check=True, timeout=15)
    # An existing process still executes the user's old helper until restarted.
    for name in UNITS:
        if not (backup / 'units' / name).is_file():
            continue
        subprocess.run(['systemctl', '--user', 'try-restart', name], check=True, timeout=30)
    if Path('/usr/share/gnome-shell/extensions/nodalix-startup-cursor@local/metadata.json').is_file():
        enable_startup_cursor(settings)
    Gio.Settings.sync()
    marker.write_text('0.3.1\n')


if __name__ == '__main__':
    main()
