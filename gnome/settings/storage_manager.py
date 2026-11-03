#!/usr/bin/python3
"""Measure storage on demand and send selected items to the native Trash."""
from __future__ import annotations
import argparse
import heapq
import json
import os
from pathlib import Path
import shutil
import stat
import sys

from gi.repository import Gio, GLib

HOME = Path.home()
PACMAN = Path('/var/lib/pacman/local')
CATEGORIES = ('apps', 'documents', 'music', 'media', 'downloads', 'development')
LOCATIONS = ('DOCUMENTS', 'MUSIC', 'PICTURES', 'VIDEOS', 'DOWNLOAD', 'DESKTOP')
DEV_PATHS = ('.cargo/registry', '.rustup/toolchains', '.npm', '.gradle/caches', '.m2/repository',
             '.cache/pip', '.cache/uv', '.cache/go-build', '.local/share/pnpm/store', '.nvm/versions')
BUILD_NAMES = {'node_modules', '.venv', 'target', '.gradle', 'build', 'dist'}
AUDIO = {'.flac', '.mp3', '.m4a', '.aac', '.ogg', '.opus', '.wav', '.aiff', '.alac'}
MEDIA = {'.mp4', '.mkv', '.webm', '.mov', '.avi', '.jpg', '.jpeg', '.png', '.webp', '.heic', '.raw', '.tiff'}
LARGE = 16 * 1024**2
ROW_LIMIT = 100
SCAN_LIMIT = 350_000


def locations():
    result = {}
    for name in LOCATIONS:
        value = GLib.get_user_special_dir(getattr(GLib.UserDirectory, 'DIRECTORY_'+name))
        if value:
            path = Path(value).resolve()
            if path not in (HOME, Path('/')) and path.is_dir():
                result[path] = name
    return result


def dev_locations():
    return {HOME/name for name in DEV_PATHS if (HOME/name).is_dir() and not (HOME/name).is_symlink()}


def owned(path):
    try:
        return path.lstat().st_uid == os.getuid() and not path.is_symlink()
    except OSError:
        return False


def walk(root, budget, dev_folder=None, seen=None):
    """Visit local regular files without following links or counting an inode twice."""
    if seen is None:
        seen = set()
    try:
        device = root.stat().st_dev
    except OSError:
        return
    folders = [root]
    while folders and budget[0] > 0:
        folder = folders.pop()
        try:
            with os.scandir(folder) as children:
                for child in children:
                    if budget[0] <= 0:
                        return
                    budget[0] -= 1
                    try:
                        info = child.stat(follow_symlinks=False)
                    except OSError:
                        continue
                    if info.st_dev != device or child.name in ('.git', '.svn'):
                        continue
                    path = Path(child.path)
                    if stat.S_ISDIR(info.st_mode):
                        if dev_folder and child.name in BUILD_NAMES and owned(path):
                            dev_folder(path)
                        else:
                            folders.append(path)
                    elif stat.S_ISREG(info.st_mode):
                        key = (info.st_dev, info.st_ino)
                        if info.st_nlink > 1:
                            if key in seen:
                                continue
                            seen.add(key)
                        yield path, info
        except OSError:
            continue


def directory_size(path, budget, seen=None):
    return sum(info.st_size for _, info in walk(path, budget, seen=seen))


def file_category(path, location):
    if location == 'DOWNLOAD':
        return 'downloads'
    if location == 'MUSIC' or path.suffix.casefold() in AUDIO:
        return 'music'
    if location in ('PICTURES', 'VIDEOS') or path.suffix.casefold() in MEDIA:
        return 'media'
    return 'documents'


def file_inventory(roots=None, dev=None, limit=SCAN_LIMIT):
    roots = locations() if roots is None else roots
    dev = dev_locations() if dev is None else dev
    totals = dict.fromkeys(CATEGORIES[1:], 0)
    rows = {kind: [] for kind in totals}
    seen = set()
    partial = False
    sequence = 0

    def add(kind, path, size, show=True):
        nonlocal sequence
        totals[kind] += size
        if not show or not owned(path):
            return
        try:
            str(path).encode('utf-8')
        except UnicodeEncodeError:
            return
        sequence += 1
        row = {'name': path.name, 'path': str(path), 'detail': str(path.parent), 'size': size}
        item = (size, sequence, row)
        if len(rows[kind]) < ROW_LIMIT:
            heapq.heappush(rows[kind], item)
        elif size > rows[kind][0][0]:
            heapq.heapreplace(rows[kind], item)

    def add_dev(path, allowance):
        nonlocal partial
        budget = [allowance]
        add('development', path, directory_size(path, budget, seen))
        partial |= budget[0] <= 0

    # Keep a large project or cache from consuming the budget for other
    # categories. Bound traversal independently in each personal folder.
    top = [(root, location) for root, location in roots.items() if not any(parent in roots for parent in root.parents)]
    for root, location in top:
        budget = [max(1, limit//max(1, len(top)))]
        def project_output(path):
            allowance = min(10_000, max(1, budget[0]//8))
            budget[0] -= allowance
            add_dev(path, allowance)
        for path, info in walk(root, budget, dev_folder=project_output, seen=seen):
            selected = next((roots[p] for p in path.parents if p in roots), location)
            kind = file_category(path, selected)
            add(kind, path, info.st_size, kind != 'documents' or info.st_size >= LARGE)
        partial |= budget[0] <= 0
    for path in sorted(dev):
        if not any(path.is_relative_to(root) for root in roots):
            add_dev(path, min(50_000, max(1, limit//max(1, len(dev)))))
    return {key: [item[2] for item in sorted(values, reverse=True)] for key, values in rows.items()}, totals, partial


def package_sizes(db=PACMAN):
    result = {}
    for package in db.iterdir() if db.is_dir() else ():
        try:
            desktops = [line.removeprefix('usr/share/applications/') for line in (package/'files').read_text().splitlines()
                        if line.startswith('usr/share/applications/') and line.endswith('.desktop')]
            if not desktops:
                continue
            desc = (package/'desc').read_text().splitlines()
            size = int(desc[desc.index('%SIZE%')+1]); name = desc[desc.index('%NAME%')+1]
            for desktop in desktops:
                result[desktop.replace('/', '-')] = (name, size)
        except (OSError, ValueError, IndexError, UnicodeError):
            continue
    return result


def flatpak_sizes():
    result = {}
    try:
        import gi
        gi.require_version('Flatpak', '1.0')
        from gi.repository import Flatpak
        for install in [Flatpak.Installation.new_user(None), *Flatpak.get_system_installations(None)]:
            for ref in install.list_installed_refs(None):
                if ref.format_ref().startswith('app/'):
                    result.setdefault(ref.get_name(), ref.get_installed_size())
    except (ImportError, ValueError, GLib.Error):
        pass
    return result


def app_library():
    sys.path.insert(0, '/usr/lib/nodalix-apps')
    import library
    return library


def application_inventory():
    library = app_library()
    registry = library.read_json(library.STATE/'registry.json')
    folder = library.applications_folder()
    packages = package_sizes()
    flatpaks = flatpak_sizes()
    result, used = [], set()
    for ident, record in registry.items():
        if not (folder/record['filename']).is_file():
            continue
        app = library.desktop_app(ident)
        if not app:
            continue
        image = library.appimage_path(app)
        flatpak = app.get_string('X-Flatpak')
        backend, size, key = 'Aplicación', None, ident
        if image:
            backend, size, key = 'AppImage', image.stat().st_size, str(image)
        elif flatpak:
            backend, size, key = 'Flatpak', flatpaks.get(flatpak), flatpak
        elif ident in packages:
            key, size = packages[ident]
            backend = 'Paquete del sistema'
        else:
            target = library.provider(app)
            if target['kind'] == 'nodalix':
                backend = 'Nodalix Apps'
                size = directory_size(Path(target['root']), [SCAN_LIMIT])
            elif target['kind'] == 'steam':
                backend = 'Steam'
        if key in used:
            continue
        used.add(key)
        result.append({'name': app.get_display_name(), 'id': ident, 'detail': backend,
                       'size': size, 'icon': app.get_icon().to_string() if app.get_icon() else ''})
    return sorted(result, key=lambda row: (-(row['size'] if row['size'] is not None else -1), row['name'].casefold()))


def scan():
    files, totals, partial = file_inventory()
    apps = application_inventory()
    totals['apps'] = sum(row['size'] or 0 for row in apps)
    volume = shutil.disk_usage(HOME)
    return {'volume': {'total': volume.total, 'used': volume.used, 'free': volume.free},
            'categories': {**files, 'apps': apps}, 'totals': totals, 'partial': partial}


def trash_file(path):
    original = Path(path)
    resolved = original.resolve(strict=True)
    if original.is_symlink() or not owned(resolved) or resolved.is_mount():
        raise ValueError('No se puede enviar este elemento a la papelera.')
    if not (resolved in dev_locations() or any(resolved != root and resolved.is_relative_to(root) for root in locations())):
        raise ValueError('El elemento está fuera de las carpetas de almacenamiento administradas.')
    Gio.File.new_for_path(str(resolved)).trash(None)


def trash_app(ident):
    library = app_library()
    record = library.read_json(library.STATE/'registry.json').get(ident)
    if not record:
        raise ValueError('La aplicación ya no está disponible.')
    folder = library.applications_folder()
    path = folder/record['filename']
    if path.parent != folder or library.descriptor(path).get('token') != record['token']:
        raise ValueError('El archivo de la aplicación ha cambiado. Actualiza la lista.')
    Gio.File.new_for_path(str(path)).trash(None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('scan', 'trash-file', 'trash-app'))
    parser.add_argument('target', nargs='?')
    args = parser.parse_args()
    try:
        if args.action == 'scan':
            print(json.dumps(scan(), ensure_ascii=False))
        elif args.action == 'trash-file':
            trash_file(args.target)
        else:
            trash_app(args.target)
    except (OSError, ValueError, GLib.Error) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
