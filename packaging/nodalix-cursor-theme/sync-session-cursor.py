#!/usr/bin/python3
"""Expose the packaged cursor to Steam runtimes that only mount user icons."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

SYSTEM_THEME = Path('/usr/share/nodalix/cursor-theme/Nodalix')


def xdg(name, fallback):
    directory = Path(os.environ.get(name, ''))
    return directory if directory.is_absolute() else Path.home() / fallback


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def snapshot(directory, ancestors=()):
    if directory in ancestors:
        raise ValueError(f'Cyclic cursor alias: {directory}')
    ancestors = (*ancestors, directory)
    files = {}
    for root, dirs, names in os.walk(directory):
        parent = Path(root)
        # Dereference only aliases contained within the theme being compared.
        for name in list(dirs):
            path = parent / name
            if path.is_symlink():
                dirs.remove(name)
                real = path.resolve(strict=True)
                if not real.is_relative_to(directory) or not real.is_dir():
                    raise ValueError(f'External cursor alias: {path}')
                for key, value in snapshot(real, ancestors).items():
                    files[str(path.relative_to(directory) / key)] = value
        for name in names:
            path = parent / name
            if not path.resolve(strict=True).is_relative_to(directory) or not path.is_file():
                raise ValueError(f'External cursor file: {path}')
            files[str(path.relative_to(directory))] = digest(path)
    return files


def index_selects_nodalix(directory):
    index = directory / 'index.theme'
    if index.is_symlink() or not index.is_file():
        return False
    for line in index.read_text(encoding='utf-8').splitlines():
        if line.startswith('Inherits='):
            return line.split('=', 1)[1].split(',', 1)[0].strip() == 'Nodalix'
    # A physical copy of the Nodalix theme also works as Xcursor's default.
    return False


def matches_installed(current, expected, default=False):
    for name in ('index.theme', 'cursor.theme'):
        current.pop(name, None)
    expected = {key: value for key, value in expected.items() if key != 'index.theme'}
    raster = {key: value for key, value in expected.items() if key.startswith('cursors/')}
    if default and not current:
        return True
    return current in (expected, raster)


def synchronize(source, icons, state):
    if not (source / 'cursors').is_dir():
        return
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    icons.mkdir(parents=True, exist_ok=True)
    with (state / 'sync.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        generation = (source / 'index.theme').stat()
        token = [generation.st_dev, generation.st_ino, generation.st_mtime_ns]
        expected = None
        for name in ('Nodalix', 'default'):
            target = icons / name
            receipt = state / (name + '.json')
            try:
                previous = json.loads(receipt.read_text(encoding='utf-8'))
            except (OSError, ValueError):
                previous = {}
            if target.is_symlink() or (target.exists() and not target.is_dir()):
                continue
            if target.is_dir() and previous.get('generation') == token:
                continue
            expected = expected if expected is not None else snapshot(source)
            if target.is_dir():
                try:
                    current = snapshot(target)
                except (OSError, ValueError):
                    continue
                named_copy = (target / 'index.theme').is_file() and (
                    digest(target / 'index.theme') == expected.get('index.theme')
                    or (name == 'default' and index_selects_nodalix(target)))
                if current != previous.get('files') and not (
                        named_copy and matches_installed(current.copy(), expected, name == 'default')):
                    continue
            with tempfile.TemporaryDirectory(prefix='.nodalix-cursor-', dir=icons) as folder:
                stage = Path(folder) / name
                # Steam may not see system paths. Keep a self-contained copy of the theme.
                if name == 'default':
                    stage.mkdir()
                    shutil.copytree(source / 'cursors', stage / 'cursors', symlinks=True)
                    shutil.copyfile(source.parent / 'default/index.theme', stage / 'index.theme')
                else:
                    shutil.copytree(source, stage, symlinks=True)
                record = dict(generation=token, files=snapshot(stage))
                prior = stage.with_name('previous')
                if target.exists():
                    target.rename(prior)
                try:
                    stage.rename(target)
                except OSError:
                    if prior.exists():
                        prior.rename(target)
                    raise
                temporary = receipt.with_suffix('.new')
                temporary.write_text(json.dumps(record), encoding='utf-8')
                temporary.replace(receipt)


def main():
    try:
        result = subprocess.run(
            ['gsettings', 'get', 'org.gnome.desktop.interface', 'cursor-theme'],
            check=True, capture_output=True, text=True, timeout=5)
        if result.stdout.strip() != "'Nodalix'":
            return
        synchronize(SYSTEM_THEME, xdg('XDG_DATA_HOME', '.local/share') / 'icons',
                    xdg('XDG_STATE_HOME', '.local/state') / 'nodalix/cursor-theme')
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f'Nodalix cursor: {error}', file=sys.stderr)


if __name__ == '__main__':
    main()
