#!/usr/bin/env python3
"""Use maintained Nodalix artwork while preserving desktop launch behavior."""
import argparse
import configparser
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import stat

# Prefer the 0.5.1 hybrid theme; keep Colloid names for older installs.
THEME_VARIANTS = (
    'Nodalix-Icons',
    'Colloid-Teal',
    'Colloid-Teal-Dark',
    'Colloid-Teal-Light',
    'Colloid',
)


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def candidates(identifier, entry):
    # Settings panels are controls, not application launchers. Absolute file
    # icons cannot be displayed by GNOME's panel list (it needs GThemedIcon).
    if ('X-GNOME-Settings-Panel' in entry.get('Categories', '').split(';')
            or entry.get('Icon', '').endswith('-symbolic')):
        return []
    names = [entry.get('Icon', ''), identifier.removesuffix('.desktop')]
    try:
        command = shlex.split(entry.get('Exec', ''))
        if len(command) == 1:
            names.append(Path(command[0]).name)
    except ValueError:
        pass
    names.append(entry.get('Name', ''))
    result = []
    for name in names:
        name = Path(name).name
        name = re.sub(r'\.(svg|png|xpm)$', '', name, flags=re.I)
        name = re.sub(r'[_-]icon$', '', name, flags=re.I)
        for token in re.finditer(r'\S+', name):
            if token.start() and re.match(r'R\d{4}[ab]', token.group(), flags=re.I):
                name = name[:token.start()].rstrip()
                break
        for value in (name, name.lower(), name.removeprefix('nodalix-')):
            if value and value not in result:
                result.append(value)
    # MATLAB supplies the shared desktop identity for Simulink too.
    if any(re.fullmatch(r'(?:nodalix-)?(?:matlab|simulink)(?:[-_ ].*)?', n, re.I) for n in result):
        result += ['matlab', 'matlab-desktop']
    return result


def icon_index(roots):
    result = {}
    for root in roots:
        for variant in THEME_VARIANTS:
            folder = root / variant / 'apps/scalable'
            if folder.is_dir():
                for path in sorted(folder.iterdir()):
                    if path.suffix in ('.svg', '.png', '.xpm') and path.is_file():
                        result.setdefault(path.stem, str(path))
    return result


def read_icon_catalogue(root):
    """Read validated icon identities and hashes from the theme package."""
    try:
        catalogue = json.loads((root / 'nodalix/icon-theme/legacy-icons.json').read_text())
    except (OSError, ValueError):
        return None
    if not isinstance(catalogue, dict) or catalogue.get('schema_version') != 1:
        return None
    rows = catalogue.get('icons')
    if not isinstance(rows, list):
        return None
    result = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        name, checksum = row.get('icon'), row.get('sha256')
        if (isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9_.+-]+', name)
                and isinstance(checksum, str) and re.fullmatch(r'[0-9a-f]{64}', checksum)):
            result.append((name, checksum))
    return result


def packaged_icons(roots, theme_roots):
    legacy, preferred = {}, []
    for root in roots:
        catalogue = read_icon_catalogue(root)
        if catalogue is None:
            continue
        preferred.append(root / 'icons')
        for name, checksum in catalogue:
            legacy[checksum] = name
    ordered = dict.fromkeys(preferred + theme_roots)
    return icon_index(ordered), legacy


def choose_icon(identifier, entry, index, roots, home, legacy):
    names = candidates(identifier, entry)
    if not names:
        return None
    path = Path(os.path.normpath(entry['Icon']))
    managed = [root / folder for root in roots for folder in ('icons/hicolor', 'nodalix/applications')]
    if (path.is_absolute() and path.is_relative_to(home)
            and not any(path.is_relative_to(root) for root in managed)):
        return index.get(legacy_icon_name(path, legacy))
    themed = next((index[name] for name in names if name in index), None)
    return themed or official_craft_icon(identifier, entry, roots)


def legacy_icon_name(path, legacy):
    """Recognize small generated files without blocking on special files."""
    limit = 2 * 1024 * 1024
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, 'rb') as stream:
            metadata = os.fstat(stream.fileno())
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > limit:
                return None
            content = stream.read(limit + 1)
    except OSError:
        return None
    if len(content) > limit:
        return None
    return legacy.get(hashlib.sha256(content).hexdigest())


def replace_icon(text, icon):
    lines, in_entry = [], False
    for line in text.splitlines(keepends=True):
        if line.startswith('['):
            in_entry = line.strip() == '[Desktop Entry]'
        if in_entry and line.startswith('Icon='):
            line = 'Icon=' + icon + '\n'
        lines.append(line)
    return ''.join(lines)


def official_craft_icon(identifier, entry, roots):
    """Resolve Craft's supplied icon even when the Flatpak icon cache is stale."""
    expected = identifier.removesuffix('.desktop')
    if not (expected.startswith(('ai.storyteller.', 'com.storytold.', 'io.tangencylabs.'))
            or entry.get('X-ArtCraft-App','').lower()=='true'
            or set(entry.get('Categories','').split(';')) & {'ArtCraft','X-ArtCraft'}):
        return None
    if entry.get('Icon') != expected:
        return None  # An explicit personal icon remains authoritative.
    for root in roots:
        for size, suffix in [('scalable', '.svg'), ('512x512', '.png'),
                             ('256x256', '.png'), ('128x128', '.png')]:
            icon = root / 'icons/hicolor' / size / 'apps' / (expected + suffix)
            if icon.is_file():
                return str(icon)
    return None


def synchronize(home, data_roots=None, theme_roots=None):
    data_home = Path(os.environ.get('XDG_DATA_HOME', str(home / '.local/share')))
    state_home = Path(os.environ.get('XDG_STATE_HOME', str(home / '.local/state')))
    apps = data_home / 'applications'
    state = state_home / 'nodalix/icons'
    apps.mkdir(parents=True, exist_ok=True)
    state.mkdir(parents=True, exist_ok=True)
    if data_roots is None:
        data_roots = [Path(p) for p in os.environ.get('XDG_DATA_DIRS', '/usr/local/share:/usr/share').split(':') if p]
        data_roots += [data_home / 'flatpak/exports/share', Path('/var/lib/flatpak/exports/share')]
    roots = [data_home] + data_roots
    index, legacy = packaged_icons(roots, theme_roots or [p / 'icons' for p in roots])
    manifest = state / 'overrides.json'
    try:
        old = json.loads(manifest.read_text())
    except (OSError, ValueError):
        old = {}
    entries = {}
    # First location wins, matching XDG desktop-file precedence.
    for root in roots:
        folder = root / 'applications'
        if folder.is_dir():
            for source in sorted(folder.rglob('*.desktop')):
                identifier = str(source.relative_to(folder)).replace('/', '-')
                destination = apps / identifier
                previous = old.get(identifier, {})
                if source == destination and previous.get('source') != str(source) and previous.get('source'):
                    if digest(source.read_text()) == previous.get('written_hash'):
                        continue
                entries.setdefault(identifier, source)
    updated, chosen = {}, {}
    for identifier, source in entries.items():
        destination = apps / identifier
        previous = old.get(identifier, {})
        text = source.read_text(errors='replace')
        if source == destination and previous.get('source') == str(source):
            if digest(text) != previous.get('written_hash'):
                # Preserve independent edits; reconsider them as a new input.
                previous = {}
            else:
                text = previous.get('original', text)
        if destination.exists() and source != destination and previous:
            if digest(destination.read_text()) != previous.get('written_hash'):
                continue
        parser = configparser.ConfigParser(interpolation=None, strict=False)
        parser.optionxform = str
        try:
            parser.read_string(text)
            entry = parser['Desktop Entry']
        except (configparser.Error, KeyError):
            continue
        if entry.get('Type') != 'Application' or not entry.get('Icon'):
            continue
        icon = choose_icon(identifier, entry, index, roots, home, legacy)
        if not icon:
            # Restore only our unchanged output if a theme icon disappears.
            if previous and destination.exists() and digest(destination.read_text()) == previous.get('written_hash'):
                if source == destination:
                    destination.write_text(previous.get('original', text))
                else:
                    destination.unlink()
            continue
        replacement = replace_icon(text, icon)
        if not destination.exists() or destination.read_text() != replacement:
            temporary = destination.with_suffix('.desktop.nodalix-new')
            temporary.write_text(replacement)
            temporary.replace(destination)
        updated[identifier] = {'source': str(source), 'original': text if source == destination else '',
                               'written_hash': digest(replacement)}
        chosen[identifier] = icon
    for identifier, previous in old.items():
        source = Path(previous['source'])
        destination = apps / identifier
        if not source.exists() and destination.exists() and digest(destination.read_text()) == previous.get('written_hash'):
            destination.unlink()
    for path, text in (
        (manifest, json.dumps(updated, ensure_ascii=False, indent=2) + '\n'),
        (state / 'resolved.json', json.dumps(chosen, ensure_ascii=False) + '\n'),
    ):
        if not path.is_file() or path.read_text() != text:
            path.write_text(text)
    return chosen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--home', type=Path, default=Path.home())
    args = parser.parse_args()
    state = Path(os.environ.get('XDG_STATE_HOME', str(args.home / '.local/state'))) / 'nodalix/icons'
    state.mkdir(parents=True, exist_ok=True)
    with (state / 'lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print(json.dumps(synchronize(args.home), ensure_ascii=False))


if __name__ == '__main__':
    main()
