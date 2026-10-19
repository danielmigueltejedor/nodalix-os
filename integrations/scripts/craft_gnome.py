#!/usr/bin/python3
"""Persistent GNOME window-frame policy for the Craft suite."""
import argparse
import configparser
import json
import logging
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time

SLUGS = ('photocraft','vectorcraft','filmcraft','lightcraft','pdfcraft','effectcraft',
         'designcraft','wordcraft','deckcraft','gridcraft','cadcraft','soundcraft','figcraft','artcraft-launcher')
IDS = frozenset('ai.storyteller.' + slug for slug in SLUGS)
APP_CLASSES = frozenset(SLUGS) | IDS
BASE = Path(os.environ.get('CRAFT_GNOME_STATE', str(Path(os.environ.get('XDG_STATE_HOME', Path.home()/'.local/state'))/'nodalix-craft-gnome')))
WRAPPER = Path('/usr/bin/craft-gnome-launch')
LEGACY_WRAPPER = Path.home()/'.local/bin/craft-gnome-launch'
LOG = logging.getLogger('craft-gnome')


def parser(text):
    result = configparser.RawConfigParser(strict=False, interpolation=None)
    result.optionxform = str
    result.read_string(text)
    return result


def atomic_text(path, text, mode=None):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    prior = path.stat().st_mode & 0o777 if path.exists() else 0o600
    fd, tmp = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    try:
        os.fchmod(fd, prior if mode is None else mode)
        with os.fdopen(fd, 'w', encoding='utf-8') as out:
            out.write(text)
            out.flush()
            os.fsync(out.fileno())
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def save_json(path, data):
    atomic_text(path, json.dumps(data, indent=2, ensure_ascii=False) + '\n', 0o600)


def read_state():
    p = BASE/'launchers.json'
    if not p.exists():
        return {'entries': {}}
    return json.loads(p.read_text())


def backup_once(path, key):
    path = Path(path)
    dest = BASE/'backup'/key
    if dest.exists() or dest.with_name(dest.name+'.absent').exists():
        return
    if path.exists():
        atomic_text(dest, path.read_text(), 0o600)
    else:
        atomic_text(dest.with_name(dest.name+'.absent'), '', 0o600)


def desktop_key(path, appid, section):
    return appid + '|' + ('autostart' if path.parent.name == 'autostart' else 'applications') + '|' + section


def reconcile_desktop(path, state):
    path = Path(path)
    appid = path.stem
    if appid not in IDS or not path.is_file():
        return False
    text = path.read_text()
    data = parser(text)
    changes = {}
    changed_state = False
    # These apps do not reliably finish GNOME's startup-notification handshake.
    # Keep this policy even when an update restores StartupNotify=true, or when
    # Exec is already protected by our wrapper.
    startup_change = (data.has_section('Desktop Entry') and
                      data.get('Desktop Entry', 'StartupNotify', fallback=None) != 'false')
    for section in data.sections():
        if section != 'Desktop Entry' and not section.startswith('Desktop Action '):
            continue
        original = data.get(section, 'Exec', fallback=None)
        if original is None:
            continue
        try:
            argv = shlex.split(original)
        except ValueError:
            LOG.warning('Ignoring invalid desktop command in %s [%s]', path, section)
            continue
        if not argv:
            continue
        key = desktop_key(path, appid, section)
        if Path(argv[0]) in (WRAPPER, LEGACY_WRAPPER):
            # Keep the original target when retiring the old per-user wrapper.
            if Path(argv[0]) == LEGACY_WRAPPER and key in state['entries']:
                changes[section] = original.replace(str(LEGACY_WRAPPER), str(WRAPPER), 1)
            continue
        entry = {'appid': appid, 'section': section, 'exec': original,
                 'icon': data.get('Desktop Entry','Icon',fallback=appid),
                 'name': data.get('Desktop Entry','Name',fallback=appid), 'desktop': str(path)}
        if state['entries'].get(key) != entry:
            state['entries'][key] = entry
            changed_state = True
        field = next((a for a in argv if a in ('%F','%U','%f','%u')), '')
        command = '"' + str(WRAPPER) + '" "' + key + '"' + (' ' + field if field else '')
        changes[section] = command
    if not changes and not startup_change:
        return False
    # Persist targets before publishing Exec lines that refer to them.
    if changed_state:
        save_json(BASE/'launchers.json', state)
    section = None
    lines = []
    for line in text.splitlines(keepends=True):
        m = re.match(r'^\[([^\]]+)\]', line)
        if m:
            section = m.group(1)
            if section == 'Desktop Entry' and startup_change:
                lines.append(line if line.endswith('\n') else line + '\n')
                lines.append('StartupNotify=false\n')
                continue
        if section == 'Desktop Entry' and startup_change and re.match(r'^StartupNotify\s*=', line):
            continue
        if section in changes and line.startswith('Exec='):
            line = 'Exec=' + changes[section] + '\n'
        lines.append(line)
    backup_once(path, 'desktops/' + ('autostart-' if path.parent.name=='autostart' else '') + path.name)
    atomic_text(path, ''.join(lines))
    LOG.info('Protected launch command: %s', path.name)
    return True


def command_for(entry, args):
    result = []
    used = False
    for token in shlex.split(entry['exec']):
        if token in ('%F','%U'):
            result.extend(args); used = True
        elif token in ('%f','%u'):
            result.extend(args[:1]); used = True
        elif token == '%i':
            if entry.get('icon'):
                result.extend(['--icon',entry['icon']])
        elif token in ('%d','%D','%n','%N','%v','%m'):
            continue
        else:
            # File arguments are inserted above, never parsed as command text.
            token = token.replace('%%','\x00').replace('%c',entry.get('name','')).replace('%k',entry.get('desktop','')).replace('\x00','%')
            result.append(token)
    if not used:
        result.extend(args)
    if not result or Path(result[0]) == WRAPPER:
        raise ValueError('Invalid or recursive stored launch command')
    return result


def reconcile_steam_startup(path):
    """Keep Steam's startup feedback off without wrapping its launch command."""
    path = Path(path)
    if path.name != 'steam.desktop' or not path.is_file():
        return False
    text = path.read_text()
    data = parser(text)
    if (not data.has_section('Desktop Entry') or
            data.get('Desktop Entry', 'StartupNotify', fallback=None) == 'false'):
        return False
    lines = []
    section = None
    for line in text.splitlines(keepends=True):
        match = re.match(r'^\[([^\]]+)\]', line)
        if match:
            section = match.group(1)
            if section == 'Desktop Entry':
                lines.append(line if line.endswith('\n') else line + '\n')
                lines.append('StartupNotify=false\n')
                continue
        if section == 'Desktop Entry' and re.match(r'^StartupNotify\s*=', line):
            continue
        lines.append(line)
    backup_once(path, 'desktops/steam.desktop')
    atomic_text(path, ''.join(lines))
    LOG.info('Disabled unfinished Steam startup feedback')
    return True


def force_preference(path, section, key):
    path = Path(path)
    if not path.is_file():
        return False
    text = path.read_text()
    data = json.loads(text)
    if not isinstance(data, dict):
        return False
    group = data.get(section)
    if group is None:
        group = {}; data[section] = group
    if not isinstance(group, dict):
        return False
    if group.get(key) is True:
        return False
    backup_key = 'preferences/' + str(path).removeprefix(str(Path.home())+'/').replace('/','__')
    backup_once(path, backup_key)
    group[key] = True
    atomic_text(path, json.dumps(data,indent=2,ensure_ascii=False)+'\n')
    LOG.info('Enabled system title bar: %s', path)
    return True


def settings_paths():
    home = Path.home()
    for slug, name, group in [('photocraft','preferences.json','interface'),('vectorcraft','ui.json','engine_prefs')]:
        for prefix in [Path(os.environ.get('XDG_CONFIG_HOME', home/'.config')), home/'.var/app'/('ai.storyteller.'+slug)/'config']:
            yield prefix/slug/name, group, 'systemTitleBar'


def app_busy(slug):
    # Do not race the editor's own autosave of preferences or UI state.
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            name = (entry/'comm').read_text().strip().lower()
        except OSError:
            continue
        if name == slug or name.startswith(slug+'-'):
            return True
    return False


def reconcile():
    state = read_state()
    changed = False
    try:
        changed |= reconcile_steam_startup(Path.home()/'.local/share/applications/steam.desktop')
    except (OSError, ValueError, configparser.Error):
        LOG.exception('Could not protect Steam startup feedback; will retry')
    for folder in [Path(os.environ.get('XDG_DATA_HOME', Path.home()/'.local/share'))/'applications', Path(os.environ.get('XDG_CONFIG_HOME', Path.home()/'.config'))/'autostart']:
        for appid in IDS:
            path = folder/(appid+'.desktop')
            try:
                changed |= reconcile_desktop(path, state)
            except (OSError, ValueError, configparser.Error):
                LOG.exception('Could not protect %s; will retry', path)
    busy = {slug: app_busy(slug) for slug in ('photocraft','vectorcraft')}
    for path, group, key in settings_paths():
        if busy.get(path.parent.name,False):
            continue
        try:
            changed |= force_preference(path, group, key)
        except (OSError,ValueError):
            LOG.warning('Could not update preferences in %s; will retry',path)
    return changed


def force_flatpak(appid):
    if appid not in IDS:
        raise ValueError('Not a Craft app')
    args = ['flatpak','override','--user','--nosocket=wayland','--nosocket=fallback-x11',
            '--socket=x11','--unset-env=WAYLAND_DISPLAY','--unset-env=WAYLAND_SOCKET']
    if appid == 'ai.storyteller.pdfcraft':
        args.append('--env=PDFCRAFT_TITLEBAR=system')
    args.append(appid)
    backup_once(Path.home()/'.local/share/flatpak/overrides'/appid, 'flatpak/'+appid)
    subprocess.run(args, check=True, timeout=15)


def refresh_graphical_environment():
    # graphical-session.target can start before GNOME imports its XWayland
    # connection into the user manager. A running daemon keeps its old environment
    # forever unless it reads that later import when it retries the connection.
    try:
        result = subprocess.run(['systemctl', '--user', 'show-environment'],
                                capture_output=True, text=True, timeout=3, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return
    if result.returncode != 0:
        return
    for line in result.stdout.splitlines():
        name, separator, value = line.partition('=')
        if separator and value and name in ('DISPLAY', 'XAUTHORITY', 'XDG_CURRENT_DESKTOP'):
            os.environ[name] = value


class FrameGuard:
    def __init__(self):
        from Xlib import X, Xatom, display
        self.X, self.Xatom = X, Xatom
        self.d = display.Display()
        self.root = self.d.screen().root
        self.atoms = {n:self.d.intern_atom(n) for n in (
            '_NET_CLIENT_LIST','_NET_WM_WINDOW_TYPE','_NET_WM_WINDOW_TYPE_NORMAL',
            '_MOTIF_WM_HINTS','_GTK_HIDE_TITLEBAR_WHEN_MAXIMIZED','_NET_WM_PID','_NET_FRAME_EXTENTS')}
        self.root.change_attributes(event_mask=X.PropertyChangeMask|X.SubstructureNotifyMask)
        self.original = {}
        self.tracked = set()
        self.scan()

    def is_craft(self, win):
        names = {s.lower() for s in (win.get_wm_class() or ())}
        if not names.intersection(APP_CLASSES):
            return False
        kind = win.get_full_property(self.atoms['_NET_WM_WINDOW_TYPE'], self.Xatom.ATOM)
        return kind is None or self.atoms['_NET_WM_WINDOW_TYPE_NORMAL'] in kind.value

    def apply(self, win):
        from Xlib import error
        try:
            if not self.is_craft(win):
                return False
            if win.id not in self.tracked:
                win.change_attributes(event_mask=self.X.PropertyChangeMask|self.X.StructureNotifyMask)
                self.tracked.add(win.id)
            motif = self.atoms['_MOTIF_WM_HINTS']
            hide = self.atoms['_GTK_HIDE_TITLEBAR_WHEN_MAXIMIZED']
            p = win.get_full_property(motif, self.X.AnyPropertyType)
            q = win.get_full_property(hide, self.X.AnyPropertyType)
            values = list(p.value) if p is not None and p.format == 32 else [0,0,0,0,0]
            values = (values + [0]*5)[:5]
            if win.id not in self.original:
                self.original[win.id] = (win, p, q)
            wanted = values.copy(); wanted[0] |= 2; wanted[2] = 1
            changed = False
            if wanted != values or p is None:
                win.change_property(motif, motif, 32, wanted)
                changed = True
            if q is None or list(q.value) != [0]:
                win.change_property(hide, self.Xatom.CARDINAL, 32, [0])
                changed = True
            if changed:
                self.d.flush()
                LOG.info('Native GNOME frame enforced on %s (%s)',hex(win.id),win.get_wm_class())
            return changed
        except (error.XError, OSError):
            return False

    def scan(self):
        p = self.root.get_full_property(self.atoms['_NET_CLIENT_LIST'],self.Xatom.WINDOW)
        if p is not None:
            live = set(int(n) for n in p.value)
            self.tracked.intersection_update(live)
            self.original = {k:v for k,v in self.original.items() if k in live}
            for xid in live:
                self.apply(self.d.create_resource_object('window',xid))
        self.d.flush()

    def events(self):
        from Xlib import error
        try:
            while self.d.pending_events():
                ev = self.d.next_event()
                if ev.type == self.X.PropertyNotify and ev.window.id in self.tracked:
                    if ev.atom in (self.atoms['_MOTIF_WM_HINTS'], self.atoms['_GTK_HIDE_TITLEBAR_WHEN_MAXIMIZED']):
                        self.apply(ev.window)
                elif ev.type in (self.X.MapNotify,self.X.CreateNotify):
                    self.apply(ev.window)
                elif ev.type == self.X.PropertyNotify and ev.window.id == self.root.id and ev.atom == self.atoms['_NET_CLIENT_LIST']:
                    self.scan()
        except (error.XError,OSError):
            LOG.exception('X11 connection failed')
            raise

    def restore(self):
        from Xlib import error
        for win,p,q in self.original.values():
            try:
                for atom,value in [(self.atoms['_MOTIF_WM_HINTS'],p),(self.atoms['_GTK_HIDE_TITLEBAR_WHEN_MAXIMIZED'],q)]:
                    if value is None:
                        win.delete_property(atom)
                    else:
                        win.change_property(atom,value.property_type,value.format,value.value)
            except error.XError:
                pass
        self.d.flush()


def daemon():
    BASE.mkdir(parents=True,exist_ok=True); BASE.chmod(0o700)
    from gi.repository import Gio, GLib, GLibUnix
    loop = GLib.MainLoop()
    frame = None
    last_scan = 0.0
    next_connect = 0.0
    monitors = []
    for folder in [Path(os.environ.get('XDG_DATA_HOME', Path.home()/'.local/share'))/'applications',Path(os.environ.get('XDG_CONFIG_HOME', Path.home()/'.config'))/'autostart']:
        folder.mkdir(parents=True,exist_ok=True)
        monitor = Gio.File.new_for_path(str(folder)).monitor_directory(Gio.FileMonitorFlags.NONE,None)
        def changed(*_):
            GLib.timeout_add(250, lambda: (reconcile(),False)[1])
        monitor.connect('changed',changed)
        monitors.append(monitor)
    reconcile()
    def tick():
        nonlocal frame, last_scan, next_connect
        now = time.monotonic()
        if frame is None:
            if now < next_connect:
                return True
            refresh_graphical_environment()
            next_connect = now + 15
        if 'GNOME' in os.environ.get('XDG_CURRENT_DESKTOP','').upper():
            try:
                if frame is None:
                    frame = FrameGuard()
                frame.events()
                if now - last_scan >= 2:
                    frame.scan(); last_scan = now
            except Exception as exc:
                LOG.warning('Window-frame connection unavailable; retrying: %s', exc)
                frame = None
                next_connect = time.monotonic() + 15
        return True
    tick()
    GLib.timeout_add(200,tick)
    GLib.timeout_add_seconds(10,lambda: (reconcile(),True)[1])
    def stop(*_):
        if frame is not None:
            frame.restore()
        loop.quit()
    GLibUnix.signal_add(GLib.PRIORITY_DEFAULT,signal.SIGTERM,stop)
    GLibUnix.signal_add(GLib.PRIORITY_DEFAULT,signal.SIGINT,stop)
    LOG.info('Persistent Craft GNOME policy active')
    loop.run()


def launch(key, args):
    state = read_state()
    entry = state['entries'].get(key)
    if not entry or entry['appid'] not in IDS:
        raise ValueError('Unknown Craft launch entry; restart nodalix-craft-gnome.service')
    # The user manager already receives the GNOME session environment at login.
    # Do not import this process's environment: it might itself be an X11-only child.
    subprocess.run(['systemctl','--user','start','nodalix-craft-gnome.service'],timeout=10,check=False)
    for path,group,name in settings_paths():
        if entry['appid'].endswith(path.parent.name) and not app_busy(path.parent.name):
            try: force_preference(path,group,name)
            except (OSError,ValueError): LOG.warning('Preferences not yet readable: %s',path)
    argv = command_for(entry,args)
    env = os.environ.copy()
    for name in ('WAYLAND_DISPLAY','WAYLAND_SOCKET'):
        env.pop(name,None)
    env['PDFCRAFT_TITLEBAR'] = 'system'
    env['ARTCRAFT_LAUNCHER_NO_DESKTOP_INTEGRATION'] = '1'
    os.execvpe(argv[0],argv,env)


def main():
    arg = argparse.ArgumentParser(description=__doc__)
    arg.add_argument('--daemon',action='store_true')
    arg.add_argument('--reconcile',action='store_true')
    arg.add_argument('--flatpaks',action='store_true')
    arg.add_argument('--launch')
    options, extra = arg.parse_known_args()
    logging.basicConfig(level=logging.INFO,format='%(asctime)s %(levelname)s %(message)s')
    if options.daemon:
        daemon()
    elif options.launch:
        launch(options.launch,extra)
    elif options.flatpaks:
        for appid in sorted(IDS-{'ai.storyteller.figcraft','ai.storyteller.artcraft-launcher'}):
            force_flatpak(appid)
    else:
        reconcile()

if __name__ == '__main__':
    main()
