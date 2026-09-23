#!/usr/bin/env python3
"""Use an available Colloid application icon, preserving desktop launch behavior."""
import argparse
import configparser
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shlex


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def candidates(identifier, entry):
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
        name = re.sub(r'\s+R\d{4}[ab].*$', '', name, flags=re.I)
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
        for variant in ('Colloid-Teal', 'Colloid-Teal-Dark', 'Colloid-Teal-Light', 'Colloid'):
            folder = root / variant / 'apps/scalable'
            if folder.is_dir():
                for path in sorted(folder.iterdir()):
                    if path.suffix in ('.svg', '.png', '.xpm') and path.is_file():
                        result.setdefault(path.stem, str(path))
    return result


def replace_icon(text, icon):
    lines, in_entry = [], False
    for line in text.splitlines(keepends=True):
        if line.startswith('['):
            in_entry = line.strip() == '[Desktop Entry]'
        if in_entry and line.startswith('Icon='):
            line = 'Icon=' + icon + '\n'
        lines.append(line)
    return ''.join(lines)


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
    index = icon_index(theme_roots or [p / 'icons' for p in roots])
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
        icon = next((index[name] for name in candidates(identifier, entry) if name in index), None)
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
    manifest.write_text(json.dumps(updated, ensure_ascii=False, indent=2) + '\n')
    (state / 'resolved.json').write_text(json.dumps(chosen, ensure_ascii=False) + '\n')
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
