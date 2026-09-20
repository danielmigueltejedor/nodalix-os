#!/usr/bin/env python3
"""Reconcile Nodalix's user layout without guessing at personal file contents."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import time

NAMES = {
    'DOCUMENTS': ('Documentos', 'Documents'), 'DOWNLOAD': ('Descargas', 'Downloads'),
    'PICTURES': ('Imágenes', 'Pictures'), 'MUSIC': ('Música', 'Music'),
    'VIDEOS': ('Vídeos', 'Videos'), 'TEMPLATES': ('Plantillas', 'Templates'),
    'PUBLICSHARE': ('Público', 'Public'),
}


def xdg(home, variable, fallback):
    value = os.environ.get(variable, '')
    return Path(value) if value.startswith('/') else home / fallback


def atomic(path, text):
    if path.is_file() and path.read_text() == text:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.nodalix-new')
    temporary.write_text(text)
    temporary.replace(path)


def preflight(source, target):
    if target.is_symlink():
        if target.resolve() == source.resolve():
            return
        raise ValueError(f'Conflicting symbolic destination: {target}')
    if not target.exists():
        return
    if source.is_dir() and not source.is_symlink() and target.is_dir():
        for child in source.iterdir():
            preflight(child, target / child.name)
    else:
        raise ValueError(f'Conflicting files: {source} / {target}')


def move(source, target):
    if target.is_symlink():
        target.unlink()
    target.parent.mkdir(parents=True, exist_ok=True)
    if not target.exists():
        shutil.move(str(source), str(target))
    else:
        for child in list(source.iterdir()):
            move(child, target / child.name)
        source.rmdir()


def reconcile(home, language, apply=False, compatibility=False):
    config = xdg(home, 'XDG_CONFIG_HOME', '.config')
    state = xdg(home, 'XDG_STATE_HOME', '.local/state') / 'nodalix/layout'
    data = xdg(home, 'XDG_DATA_HOME', '.local/share')
    cache = xdg(home, 'XDG_CACHE_HOME', '.cache')
    filename = config / 'user-dirs.dirs'
    original = filename.read_text() if filename.exists() else ''
    current = {key: Path(value.replace('$HOME', str(home))) for key, value in
               re.findall(r'^XDG_(\w+)_DIR="([^"\n]+)"', original, re.M)}
    paths, operations = {}, []
    def schedule(src, dst, alias=False):
        if src == dst or not src.exists() or src.is_symlink():
            return
        preflight(src, dst)
        operations.append((src, dst, alias))
    for key, pair in NAMES.items():
        target = home / pair[language != 'es']
        previous = current.get(key)
        if previous and (previous == home or previous.parent != home or previous.name not in pair):
            paths[key] = str(previous)
            continue
        paths[key] = str(target)
        for name in pair:
            # Localized XDG aliases protect existing application paths.
            schedule(home / name, target, True)
    docs = Path(paths['DOCUMENTS'])
    projects = docs / ('Proyectos' if language == 'es' else 'Projects')
    # The document tree can be moving in this transaction; migrate internal
    # sources on the next reconciliation rather than referring to stale paths.
    document_move = any(dst == docs for _, dst, _ in operations)
    schedule(home / 'Nodalix', projects / 'Nodalix', compatibility)
    schedule(home / 'src', projects / 'Sources', compatibility)
    for entry in sorted(home.iterdir()):
        if not entry.name.startswith('.') and entry.is_dir() and not entry.is_symlink() and (entry / '.git').is_dir():
            if entry.name not in {name for pair in NAMES.values() for name in pair} and entry.name not in ('Nodalix', 'src'):
                schedule(entry, projects / entry.name, compatibility)
    if not document_move:
        old_projects = docs / ('Projects' if language == 'es' else 'Proyectos')
        schedule(old_projects, projects, True)
    report = {'schema': 1, 'language': language, 'paths': paths,
              'projects': str(projects), 'data': str(data / 'nodalix'),
              'cache': str(cache / 'nodalix'),
              'moves': [{'from': str(s), 'to': str(d), 'compatibility': c} for s, d, c in operations]}
    if not apply:
        return report
    state.mkdir(parents=True, exist_ok=True)
    backup = state / 'migrations' / str(time.time_ns())
    if operations or original != render_xdg(home, paths):
        backup.mkdir(parents=True)
        atomic(backup / 'user-dirs.dirs', original)
        atomic(backup / 'plan.json', json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    for source, target, alias in operations:
        move(source, target)
        if alias:
            source.symlink_to(os.path.relpath(target, source.parent), target_is_directory=True)
    for value in paths.values():
        Path(value).mkdir(parents=True, exist_ok=True)
    projects.mkdir(parents=True, exist_ok=True)
    for folder in (data / 'nodalix', cache / 'nodalix/build', cache / 'nodalix/tmp', state):
        folder.mkdir(parents=True, exist_ok=True)
    (cache / 'nodalix/tmp').chmod(0o700)
    # Remove only empty, unmounted legacy scratch/desktop directories.
    for folder in list(home.glob('iphonebridge_pb_*')) + [home / 'Escritorio', home / 'Desktop']:
        if folder.is_dir() and not folder.is_symlink() and not folder.is_mount():
            try:
                folder.rmdir()
            except OSError:
                pass
    atomic(filename, render_xdg(home, paths))
    atomic(config / 'user-dirs.locale', ('es_ES' if language == 'es' else 'en_US') + '\n')
    hidden = home / '.hidden'
    lines = hidden.read_text().splitlines() if hidden.exists() else []
    managed = {n for pair in NAMES.values() for n in pair} | {'Nodalix', 'src'}
    lines = [line for line in lines if line not in managed]
    lines += [name for name in managed if (home / name).is_symlink()]
    atomic(hidden, '\n'.join(sorted(set(lines))) + '\n')
    # Kept compatible with beta-8 local integrations.
    atomic(state.parent / 'folders/paths.json', json.dumps(paths, ensure_ascii=False) + '\n')
    atomic(state / 'paths.json', json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    return report


def render_xdg(home, paths):
    lines = ['# Managed by Nodalix. Desktop is disabled.']
    for key, value in paths.items():
        value = '$HOME' + value[len(str(home)):] if value.startswith(str(home) + '/') else value
        lines.append(f'XDG_{key}_DIR="{value}"')
    return '\n'.join(lines + ['XDG_DESKTOP_DIR="$HOME"', ''])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--home', type=Path, default=Path.home())
    parser.add_argument('--language', choices=('es', 'en'))
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--compatibility', action='store_true', help='Keep aliases for legacy project paths')
    args = parser.parse_args()
    language = args.language
    if not language:
        locale = Path('/etc/locale.conf')
        text = locale.read_text() if locale.exists() else 'LANG=' + os.environ.get('LANG', 'en_US')
        language = 'es' if re.search(r'^LANG=["\']?es', text, re.M) else 'en'
    home = args.home.resolve()
    if args.apply:
        state = xdg(home, 'XDG_STATE_HOME', '.local/state') / 'nodalix/layout'
        state.mkdir(parents=True, exist_ok=True)
        with (state / 'lock').open('w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            report = reconcile(home, language, True, args.compatibility)
    else:
        report = reconcile(home, language, False, args.compatibility)
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
