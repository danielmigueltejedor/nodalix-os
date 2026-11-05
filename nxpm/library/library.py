"""Native Applications folder, installer and provider-aware Trash handling."""
from __future__ import annotations
import argparse
import base64
import fcntl
import hashlib
import json
import locale
import mmap
import os
from pathlib import Path
import re
import secrets
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading
import tomllib
import urllib.parse

import gi
gi.require_version('GdkPixbuf', '2.0')
gi.require_version('Gtk', '4.0')
from gi.repository import GdkPixbuf, Gio, GLib

HOME = Path.home()
DATA = Path(os.environ.get('XDG_DATA_HOME', HOME/'.local/share'))
STATE = Path(os.environ.get('XDG_STATE_HOME', HOME/'.local/state'))/'nodalix/applications'
STORAGE = DATA/'nodalix/applications'
DESKTOPS = DATA/'applications'
EXTENSION = '.app'
MIME = 'application/x-nodalix-app'


def read_json(path, default=None):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return {} if default is None else default


def atomic_json(path, value):
    text = json.dumps(value, ensure_ascii=False, indent=2)+'\n'
    if path.is_file() and path.read_text() == text:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix='.'+path.name, dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as output:
            output.write(text)
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)


def applications_folder():
    layout = read_json(STATE.parent/'layout/paths.json')
    if layout.get('applications'):
        folder = Path(layout['applications'])
        if folder.is_absolute() and folder.parent == HOME:
            return folder.resolve()
    for name in ('Aplicaciones', 'Applications'):
        if (HOME/name).is_dir():
            return (HOME/name).resolve()
    return HOME/('Aplicaciones' if (locale.getlocale()[0] or '').startswith('es') else 'Applications')


def notify(body):
    subprocess.run(['/usr/bin/notify-send', '--app-name=Aplicaciones', '--icon=system-software-install',
                    'Aplicaciones de Nodalix', body], check=False)


def desktop_app(ident):
    try:
        return Gio.DesktopAppInfo.new(ident)
    except TypeError:
        return None


def command(app):
    # The existing Craft wrapper stores the original Exec separately.
    text = app.get_commandline() or ''
    if 'craft-native-launch' in text or 'craft-gnome-launch' in text:
        for base in (Path(os.environ.get('XDG_STATE_HOME', HOME/'.local/state'))/'nodalix-craft-gnome', DATA/'nodalix/craft-gnome', STATE.parent/'craft-gnome'):
            entries = read_json(base/'launchers.json').get('entries', {})
            for entry in entries.values():
                if entry.get('appid') == app.get_id().removesuffix('.desktop') and entry.get('section') == 'Desktop Entry':
                    return entry.get('exec', text)
    return text


def appimage_path(app):
    explicit = app.get_string('X-Nodalix-AppImage')
    if explicit and Path(explicit).is_file():
        return Path(explicit).resolve()
    try:
        arguments = shlex.split(command(app))
    except ValueError:
        return None
    for argument in arguments:
        if argument.lower().endswith('.appimage') and Path(argument).is_absolute() and Path(argument).is_file():
            return Path(argument).resolve()
    return None


def flatpak_target(app):
    ident = app.get_string('X-Flatpak')
    if not ident:
        return None
    filename = str(Path(app.get_filename()).resolve())
    scope = '--user' if filename.startswith(str(DATA/'flatpak')+'/') else '--system'
    # User icon overrides copy the exported desktop entry. Inspect the actual
    # installations instead of inferring ownership from the override directory.
    import gi
    gi.require_version('Flatpak', '1.0')
    from gi.repository import Flatpak
    scopes = [Flatpak.Installation.new_user(None), *Flatpak.get_system_installations(None)]
    found = []
    for installation in scopes:
        for ref in installation.list_installed_refs(None):
            if ref.get_name() == ident and ref.format_ref().startswith('app/'):
                selector = '--user' if installation.get_is_user() else '--installation='+installation.get_id()
                found.append((selector, ref.format_ref()))
    if not found:
        return None
    return next((r for r in found if r[0] == scope), found[0])


def system_desktop(ident):
    if not re.fullmatch(r'[\w.+@ -]+\.desktop', ident) or '/' in ident:
        return None
    for base in (Path('/usr/share/applications'), Path('/usr/local/share/applications')):
        path = base/ident
        if path.is_file() and path.stat().st_uid == 0:
            return path
    return None


def provider(app):
    image = appimage_path(app)
    if image:
        return {'kind': 'appimage', 'path': str(image)}
    target = flatpak_target(app)
    if target:
        return {'kind': 'flatpak', 'scope': target[0], 'ref': target[1]}
    text = command(app)
    for manifest in Path('/usr/share/nodalix-apps/apps').glob('*.toml'):
        data = tomllib.loads(manifest.read_text())
        root = Path(data['root']).expanduser()
        if (root/'current').exists() and (str(root) in text or data['id'] in Path(app.get_executable() or '').name):
            return {'kind': 'nodalix', 'root': str(root), 'id': data['id']}
    steam = re.search(r'steam://rungameid/(\d+)', text)
    if steam:
        return {'kind': 'steam', 'id': steam[1]}
    if system_desktop(app.get_id()):
        return {'kind': 'package', 'desktop_id': app.get_id()}
    # A generated web application or personal launcher has no independent
    # package to remove. Remove its launcher; never delete an inferred binary.
    return {'kind': 'launcher'}


def icon_uri(app):
    return resolve_icon(app.get_icon())


def resolve_icon(icon):
    if isinstance(icon, Gio.FileIcon):
        file = icon.get_file()
        if file.query_exists(None):
            return file.get_uri()
    if icon:
        from gi.repository import Gtk
        theme = Gtk.IconTheme.new()
        try:
            theme.set_theme_name(Gio.Settings.new('org.gnome.desktop.interface').get_string('icon-theme'))
        except GLib.Error:
            theme.set_theme_name('Adwaita')
        paintable = theme.lookup_by_gicon(icon, 128, 1, Gtk.TextDirection.NONE, Gtk.IconLookupFlags.FORCE_REGULAR)
        if paintable and paintable.get_file():
            return paintable.get_file().get_uri()
    return ''


def folder_icon_uri():
    uri = resolve_icon(Gio.ThemedIcon.new('folder'))
    if not uri:
        return ''
    source = Gio.File.new_for_uri(uri).get_path()
    if not source:
        return uri
    source = Path(source)
    mime = {'.svg': 'image/svg+xml', '.png': 'image/png'}.get(source.suffix.lower())
    if not mime:
        return uri
    try:
        # Reuse the installed theme's folder silhouette and color. The emblem
        # stays a vector even when the theme supplies a raster base.
        with source.open('rb') as icon:
            artwork = icon.read(1024 * 1024 + 1)
        if not artwork or len(artwork) > 1024 * 1024:
            return uri
        style = folder_emblem_style(artwork)
        encoded = base64.b64encode(artwork).decode('ascii')
        svg = f'''<svg xmlns="http://www.w3.org/2000/svg"
     xmlns:xlink="http://www.w3.org/1999/xlink" width="128" height="128" viewBox="0 0 128 128">
  <image width="128" height="128" xlink:href="data:{mime};base64,{encoded}"/>
  <g {style}>
    <rect x="49" y="61" width="13" height="13" rx="3"/>
    <rect x="66" y="61" width="13" height="13" rx="3"/>
    <rect x="49" y="78" width="13" height="13" rx="3"/>
    <rect x="66" y="78" width="13" height="13" rx="3"/>
  </g>
</svg>
'''.encode('ascii')
        # Give each rendition its own URI so file managers can invalidate
        # a cached pixbuf when the theme changes.
        target = STORAGE/f'folder-applications-{hashlib.sha256(svg).hexdigest()[:12]}.svg'
        if not target.is_file():
            STORAGE.mkdir(parents=True, exist_ok=True)
            GLib.file_set_contents(str(target), svg)
        return target.as_uri()
    except (OSError, GLib.Error):
        return uri


def folder_emblem_style(artwork):
    stream = Gio.MemoryInputStream.new_from_bytes(GLib.Bytes.new(artwork))
    pixbuf = GdkPixbuf.Pixbuf.new_from_stream_at_scale(stream, 128, 128, True, None)
    pixels = pixbuf.get_pixels()
    channels = pixbuf.get_n_channels()

    def color_at(x, y):
        offset = int(y * pixbuf.get_height()) * pixbuf.get_rowstride()
        offset += int(x * pixbuf.get_width()) * channels
        return pixels[offset:offset + channels]

    # Native folders use the darker tab as an accent against the front face.
    tab = color_at(0.1875, 0.21875)
    face = color_at(0.5, 0.59375)
    if (channels == 4 and tab[3] < 240) or max(abs(a - b) for a, b in zip(tab[:3], face[:3])) < 24:
        return 'fill="#000" opacity=".42"'
    return 'fill="#{:02x}{:02x}{:02x}"'.format(*tab[:3])


def set_icon(path, uri):
    if not uri or not path.exists():
        return
    file = Gio.File.new_for_path(str(path))
    try:
        old = file.query_info('metadata::custom-icon', Gio.FileQueryInfoFlags.NONE, None)
        if old.get_attribute_string('metadata::custom-icon') != uri:
            file.set_attribute_string('metadata::custom-icon', uri, Gio.FileQueryInfoFlags.NONE, None)
    except GLib.Error:
        # GVfs metadata is not available in headless package builds.
        pass


def safe_name(name):
    return re.sub(r'[/\\\x00-\x1f]', '_', name).strip('. ')[:120] or 'Aplicación'


def descriptor(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 4096:
        return {}
    return read_json(path)


def trash_requests(folder, registry, trash_data=DATA/'Trash'):
    """Only a real Trash move from this folder, with a registered token, counts."""
    import configparser
    result = []
    for info in (trash_data/'info').glob('*.trashinfo'):
        try:
            config = configparser.ConfigParser(interpolation=None)
            config.read(info)
            original = Path(urllib.parse.unquote(config['Trash Info']['Path']))
            if original.parent != folder:
                continue
            file = trash_data/'files'/info.name.removesuffix('.trashinfo')
            value = descriptor(file)
            record = registry.get(value.get('desktop_id'))
            if record and secrets.compare_digest(str(value.get('token', '')), record['token']):
                result.append((value['desktop_id'], file, info))
        except (OSError, KeyError, ValueError, configparser.Error):
            continue
    return result


def reconcile(folder, registry, apps=None, pending=()):
    folder.mkdir(parents=True, exist_ok=True)
    set_icon(folder, folder_icon_uri())
    apps = Gio.AppInfo.get_all() if apps is None else apps
    present = {}
    for file in folder.glob('*'+EXTENSION):
        value = descriptor(file)
        record = registry.get(value.get('desktop_id'))
        if record and value.get('token') == record['token']:
            present.setdefault(value['desktop_id'], file)
    visible = set()
    for app in apps:
        if not isinstance(app, Gio.DesktopAppInfo) or not Path(app.get_filename()).is_file() or not app.should_show() or app.get_id() == 'com.nodalix.Applications.desktop':
            continue
        ident = app.get_id()
        visible.add(ident)
        if ident in pending:
            continue
        record = registry.setdefault(ident, {'token': secrets.token_hex(32)})
        path = present.get(ident)
        if path is None:
            path = folder/(safe_name(app.get_display_name())+EXTENSION)
            if path.exists():
                path = folder/(safe_name(app.get_display_name())+' · '+hashlib.sha256(ident.encode()).hexdigest()[:8]+EXTENSION)
            if path.exists() and descriptor(path).get('token') != record['token']:
                continue  # Never overwrite personal files.
        atomic_json(path, {'format': 'nodalix-app-1', 'desktop_id': ident, 'token': record['token']})
        record.update(filename=path.name, name=app.get_display_name())
        uri = icon_uri(app)
        set_icon(path, uri)
        image = appimage_path(app)
        if image and image.parent == folder:
            set_icon(image, uri)
    for ident in list(registry):
        if ident not in visible and ident not in pending:
            path = present.get(ident)
            if path:
                path.unlink()
            del registry[ident]
    return len(visible)


def uninstall(app, target, runner=subprocess.run):
    kind = target['kind']
    if kind == 'flatpak':
        result = runner(['/usr/bin/flatpak', 'uninstall', target['scope'], '--noninteractive', '-y', '--', target['ref']])
        if result.returncode:
            raise RuntimeError('Flatpak no pudo desinstalar la aplicación.')
    elif kind == 'package':
        result = runner(['/usr/bin/pkexec', '/usr/lib/nodalix-apps/remove-system-app', target['desktop_id']])
        if result.returncode:
            raise RuntimeError('Se canceló la desinstalación o el sistema necesita este paquete.')
    elif kind == 'steam':
        # Steam owns game content and its final removal prompt.
        Gio.AppInfo.launch_default_for_uri('steam://uninstall/'+target['id'], None)
        raise RuntimeError('Completa la desinstalación en Steam. La app seguirá visible mientras esté instalada.')
    elif kind in ('appimage', 'nodalix'):
        path = Path(target['path'] if kind == 'appimage' else target['root'])
        if not path.is_relative_to(HOME) or path == HOME or (kind == 'nodalix' and not path.is_relative_to(HOME/'.local/opt/nodalix/apps')):
            raise RuntimeError('Esta instalación requiere su gestor original.')
        if path.exists():
            Gio.File.new_for_path(str(path)).trash(None)
        if kind == 'appimage':
            installed = read_json(STATE/'appimages.json')
            for root in installed.pop(app.get_id(), []):
                managed = Path(root)
                if managed.parent == STORAGE and re.fullmatch(r'[0-9a-f]{64}', managed.name) and managed.exists():
                    Gio.File.new_for_path(str(managed)).trash(None)
            atomic_json(STATE/'appimages.json', installed)
            folder = applications_folder()
            for alias in folder.glob('*.AppImage'):
                if alias.is_symlink() and alias.resolve() == path:
                    alias.unlink()
    elif kind != 'launcher':
        raise RuntimeError('Tipo de instalación desconocido.')
    # Remove all user overrides, leaving app data and shared dependencies intact.
    desktop = DESKTOPS/app.get_id()
    if desktop.exists() and not desktop.is_symlink():
        Gio.File.new_for_path(str(desktop)).trash(None)
    if kind == 'launcher' and Path(app.get_filename()).parent != DESKTOPS:
        raise RuntimeError('La aplicación necesita su gestor original.')


def squashfs_offset(path):
    with path.open('rb') as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as data:
        # Type 2 AppImages embed a standard SquashFS image. Never run downloaded
        # code merely to extract its icon or desktop information.
        if data[:4] != b'\x7fELF' or data[8:11] != b'AI\x02':
            raise ValueError('Se necesita un AppImage de tipo 2 válido.')
        start = 0
        while True:
            offset = data.find(b'hsqs', start)
            if offset < 0:
                raise ValueError('El AppImage no contiene un sistema de archivos válido.')
            if offset+96 <= len(data) and int.from_bytes(data[offset+28:offset+30], 'little') == 4:
                return offset
            start = offset+4


def import_appimage(source, folder):
    STATE.mkdir(parents=True, exist_ok=True)
    source = source.resolve(strict=True)
    if source.parent == folder and not (STATE/'adopted-v1.done').exists():
        backup = STATE/'before-040/payloads'/source.name
        backup.parent.mkdir(parents=True, exist_ok=True)
        if not backup.exists():
            try:
                os.link(source, backup)
            except OSError:
                shutil.copy2(source, backup)
    if not source.is_file() or source.is_symlink():
        raise ValueError('Selecciona un archivo AppImage local.')
    digest = hashlib.file_digest(source.open('rb'), 'sha256').hexdigest()
    dest = STORAGE/digest
    dest.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='appimage-', dir=STATE) as temporary:
        extracted = Path(temporary)/'image'
        offset = squashfs_offset(source)
        subprocess.run(['/usr/bin/unsquashfs', '-no-progress', '-no-xattrs', '-o', str(offset),
                        '-d', str(extracted), str(source), '*.desktop', '.DirIcon', '*.png', '*.svg'],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=60)
        launchers = list(extracted.glob('*.desktop'))
        if not launchers:
            raise ValueError('El AppImage no incluye información de aplicación.')
        def extract_link(file):
            resolved = file.resolve()
            if not resolved.is_relative_to(extracted):
                raise ValueError('El AppImage contiene un enlace fuera de su instalación.')
            if not resolved.exists() and file.is_symlink():
                subprocess.run(['/usr/bin/unsquashfs', '-f', '-no-progress', '-no-xattrs', '-o', str(offset),
                                '-d', str(extracted), str(source), str(resolved.relative_to(extracted))],
                               check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=60)
            return resolved
        launcher = extract_link(launchers[0])
        key = GLib.KeyFile()
        key.load_from_file(str(launcher), GLib.KeyFileFlags.NONE)
        ident = launchers[0].name
        existing = next((a for a in Gio.AppInfo.get_all() if isinstance(a, Gio.DesktopAppInfo) and appimage_path(a) == source), None)
        if existing:
            ident = existing.get_id()
        elif desktop_app(ident) and not appimage_path(desktop_app(ident)):
            ident = 'nodalix-appimage.'+ident
        if not re.fullmatch(r'[\w.+@ -]+\.desktop', ident):
            raise ValueError('El identificador de la aplicación no es válido.')
        binary = dest/source.name
        if binary != source:
            temporary_binary = dest/('.'+source.name+'.part')
            shutil.copy2(source, temporary_binary)
            if hashlib.file_digest(temporary_binary.open('rb'), 'sha256').hexdigest() != digest:
                temporary_binary.unlink(missing_ok=True)
                raise ValueError('El archivo cambió mientras se copiaba. Vuelve a intentarlo cuando termine la descarga.')
            temporary_binary.replace(binary)
        binary.chmod(0o755)
        icon_name = key.get_string('Desktop Entry', 'Icon') if 'Icon' in key.get_keys('Desktop Entry')[0] else ''
        candidates = [extracted/'.DirIcon'] + [p for p in extracted.rglob('*') if p.is_file() and p.suffix.lower() in ('.svg', '.png') and p.stem == Path(icon_name).stem]
        for candidate in candidates:
            resolved = extract_link(candidate)
            if resolved.is_relative_to(extracted) and resolved.is_file() and resolved.stat().st_size < 16*1024**2:
                icon = dest/('icon'+(resolved.suffix if resolved.suffix.lower() in ('.svg', '.png') else '.png'))
                shutil.copy2(resolved, icon)
                key.set_string('Desktop Entry', 'Icon', str(icon))
                break
        # Do not carry an upstream Exec, action or DBus service into a local
        # installation. Activation always runs this exact copied AppImage.
        for group in key.get_groups()[0]:
            if group != 'Desktop Entry':
                key.remove_group(group)
        for option in ('Actions', 'TryExec', 'Path', 'Hidden', 'OnlyShowIn', 'NotShowIn'):
            if option in key.get_keys('Desktop Entry')[0]:
                key.remove_key('Desktop Entry', option)
        key.set_boolean('Desktop Entry', 'DBusActivatable', False)
        key.set_boolean('Desktop Entry', 'NoDisplay', False)
        key.set_string('Desktop Entry', 'Exec', '"'+str(binary).replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%')+'" %U')
        key.set_string('Desktop Entry', 'X-Nodalix-AppImage', str(binary))
        DESKTOPS.mkdir(parents=True, exist_ok=True)
        target = DESKTOPS/ident
        # Preserve a customized launcher. Known AppImage launchers get only
        # their payload path updated; the original file is backed up once.
        if target.exists():
            backup = STATE/'launcher-backups'/ident
            backup.parent.mkdir(parents=True, exist_ok=True)
            if not backup.exists():
                shutil.copy2(target, backup)
        if not existing:
            key.save_to_file(str(target))
        installed = read_json(STATE/'appimages.json')
        roots = installed.setdefault(ident, [])
        if str(dest) not in roots:
            roots.append(str(dest))
        atomic_json(STATE/'appimages.json', installed)
    if source.parent == folder and source != binary:
        # Keep compatibility for existing desktop Execs and external references.
        # Hide only this exact managed payload alias from the application view.
        source.unlink()
        source.symlink_to(binary)
        hidden = folder/'.hidden'
        names = set(hidden.read_text().splitlines()) if hidden.exists() else set()
        names.add(source.name)
        hidden.write_text('\n'.join(sorted(names))+'\n')
    subprocess.run(['/usr/bin/update-desktop-database', str(DESKTOPS)], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return ident


def install_file(path, folder):
    if path.suffix.lower() == '.appimage':
        return import_appimage(path, folder)
    if path.suffix.lower() in ('.flatpakref', '.flatpak'):
        subprocess.run(['/usr/bin/flatpak', 'install', '--user', '--noninteractive', '-y', '--', str(path.resolve())], check=True)
        if path.parent == folder:
            Gio.File.new_for_path(str(path)).trash(None)
        return ''
    if '.pkg.tar.' in path.name:
        subprocess.run(['/usr/bin/pkexec', '/usr/bin/pacman', '-U', '--noconfirm', '--', str(path.resolve())], check=True)
        if path.parent == folder:
            Gio.File.new_for_path(str(path)).trash(None)
        return ''
    raise ValueError('Formatos admitidos: AppImage, Flatpak y paquetes de Nodalix/Arch.')


class Library:
    def __init__(self, folder=None):
        self.folder = folder or applications_folder()
        STATE.mkdir(parents=True, exist_ok=True)
        self.folder.mkdir(parents=True, exist_ok=True)
        if not (STATE/'adopted-v1.done').exists():
            before = STATE/'before-040'
            before.mkdir(parents=True, exist_ok=True)
            inventory = before/'folder-inventory.json'
            if not inventory.exists():
                atomic_json(inventory, [{'name':p.name,'symlink':str(p.readlink()) if p.is_symlink() else None} for p in self.folder.iterdir()])
            for original in (self.folder/'.hidden', Path(os.environ.get('XDG_CONFIG_HOME', HOME/'.config'))/'mimeapps.list'):
                if original.is_file() and not (before/original.name).exists():
                    shutil.copy2(original, before/original.name)
        self._timer = 0
        self._working = False
        self._dirty = False
        self.monitors = []
        self._failed_imports = {tuple(value) for value in read_json(STATE/'failed-imports.json', [])}
        marker = STATE/'file-associations-v1.done'
        if not marker.exists():
            installed = desktop_app('com.nodalix.Applications.desktop')
            installer = desktop_app('com.nodalix.AppInstaller.desktop')
            if installed and installer:
                installed.set_as_default_for_type(MIME)
                for mime in ('application/vnd.appimage', 'application/x-iso9660-appimage', 'application/vnd.flatpak', 'application/vnd.flatpak.ref', 'application/x-alpm-package'):
                    installer.set_as_default_for_type(mime)
                marker.touch()

    def sync(self, handle_trash=True, import_files=True):
        with (STATE/'lock').open('w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            registry = read_json(STATE/'registry.json')
            requests = trash_requests(self.folder, registry)
            if handle_trash:
                for ident, trashed, info in requests:
                    app = desktop_app(ident)
                    if not app:
                        continue
                    try:
                        uninstall(app, provider(app))
                        # Retire the token, so restoring an old descriptor cannot
                        # remove a later reinstallation of the same application.
                        del registry[ident]
                        notify(registry.get(ident, {}).get('name', app.get_display_name())+' se ha desinstalado. Tus datos se conservan.')
                    except (OSError, ValueError, RuntimeError, GLib.Error, subprocess.SubprocessError) as error:
                        restore = self.folder/registry[ident]['filename']
                        if not restore.exists():
                            shutil.move(trashed, restore)
                            info.unlink(missing_ok=True)
                        notify(str(error))
                requests = trash_requests(self.folder, registry)
            if import_files:
                for file in list(self.folder.iterdir()):
                    if file.is_symlink() or not file.is_file() or file.suffix.lower() not in ('.appimage', '.flatpak', '.flatpakref') and '.pkg.tar.' not in file.name:
                        continue
                    fingerprint = (str(file), file.stat().st_size, file.stat().st_mtime_ns)
                    if fingerprint in self._failed_imports:
                        continue
                    try:
                        install_file(file, self.folder)
                    except (OSError, ValueError, GLib.Error, subprocess.SubprocessError) as error:
                        self._failed_imports.add(fingerprint)
                        atomic_json(STATE/'failed-imports.json', sorted(self._failed_imports))
                        notify('No se pudo instalar '+file.name+': '+str(error)[:160])
            count = reconcile(self.folder, registry, pending=[r[0] for r in requests])
            atomic_json(STATE/'registry.json', registry)
            if import_files and not self._failed_imports:
                (STATE/'adopted-v1.done').touch()
            return count

    def schedule(self, *_):
        if self._timer:
            GLib.source_remove(self._timer)
        self._timer = GLib.timeout_add(1500, self._start)

    def _start(self):
        self._timer = 0
        if self._working:
            self._dirty = True
            return GLib.SOURCE_REMOVE
        self._working = True
        def work():
            try:
                subprocess.run([sys.executable, __file__, 'sync'], check=True, stdout=subprocess.DEVNULL)
            except Exception as error:
                print('Applications sync failed:', error, file=sys.stderr, flush=True)
            GLib.idle_add(done)
        def done():
            self._working = False
            if self._dirty:
                self._dirty = False
                self.schedule()
            return GLib.SOURCE_REMOVE
        threading.Thread(target=work, daemon=True, name='applications-sync').start()
        return GLib.SOURCE_REMOVE

    def watch(self):
        paths = [self.folder, DESKTOPS, DATA/'flatpak/exports/share/applications',
                 Path('/usr/share/applications'), Path('/usr/local/share/applications'),
                 Path('/var/lib/flatpak/exports/share/applications'), DATA/'Trash/info', DATA/'Trash/files']
        for path in paths:
            if path.is_relative_to(HOME):
                path.mkdir(parents=True, exist_ok=True)
            if path.is_dir():
                monitor = Gio.File.new_for_path(str(path)).monitor_directory(Gio.FileMonitorFlags.WATCH_MOVES, None)
                monitor.connect('changed', self.schedule)
                self.monitors.append(monitor)
        self.app_monitor = Gio.AppInfoMonitor.get()
        self.app_monitor.connect('changed', self.schedule)
        settings = Gio.SettingsSchemaSource.get_default()
        interface = settings.lookup('org.gnome.desktop.interface', True) if settings else None
        if interface:
            self.icon_settings = Gio.Settings.new_full(interface, None, None)
            self.icon_settings.connect('changed::icon-theme', self.schedule)
        self.schedule()
        GLib.MainLoop().run()


def ensure_document_portal(app, attempts=4):
    ident = app.get_string('X-Flatpak')
    if not ident:
        return
    # A Flatpak launch must wait for the real document mount. Never create
    # placeholder directories or restart a portal used by open documents.
    import time
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    for attempt in range(attempts):
        try:
            value = bus.call_sync('org.freedesktop.portal.Documents', '/org/freedesktop/portal/documents',
                                  'org.freedesktop.portal.Documents', 'GetMountPoint', None, None,
                                  Gio.DBusCallFlags.NONE, 5000, None).unpack()[0]
            mount = Path(os.fsdecode(bytes(value).rstrip(b'\0')))
            if mount.is_absolute() and (mount/'by-app'/ident).is_dir():
                return
        except GLib.Error:
            pass
        if attempt+1 < attempts:
            time.sleep(.25)
    raise ValueError('El acceso de Flatpak a documentos aún no está disponible. Vuelve a abrir la aplicación en unos segundos.')


def open_app(path):
    value = descriptor(path)
    record = read_json(STATE/'registry.json').get(value.get('desktop_id'))
    if value.get('format') != 'nodalix-app-1' or not record or not secrets.compare_digest(str(value.get('token', '')), record['token']):
        raise ValueError('Este archivo no corresponde a una aplicación instalada.')
    app = desktop_app(value['desktop_id'])
    if not app:
        raise ValueError('La aplicación ya no está instalada.')
    ensure_document_portal(app)
    app.launch([], None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('sync', 'watch', 'open', 'install'))
    parser.add_argument('file', nargs='?', type=Path)
    args = parser.parse_args()
    if args.action in ('sync', 'watch'):
        library = Library()
        if args.action == 'watch':
            library.watch()
        else:
            print(library.sync())
    elif args.action == 'open':
        open_app(args.file)
    elif args.action == 'install':
        from install_ui import show_installer
        show_installer(args.file)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, GLib.Error, subprocess.SubprocessError) as error:
        notify(str(error))
        raise SystemExit(1)
