#!/usr/bin/python3
"""Session service for native Nodalix Settings pages; no compositor dependency."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import threading
import time

from gi.repository import Gio, GLib

BUS = 'com.nodalix.Settings'
PATH = '/com/nodalix/Settings'
INTERFACE = 'com.nodalix.Settings1'
XML = '''<node><interface name="com.nodalix.Settings1">
<method name="GetUpdates"><arg type="s" direction="out"/></method>
<method name="CheckUpdates"/>
<method name="StartUpdate"><arg type="s" direction="in"/></method>
<method name="GetWallpapers"><arg type="s" direction="out"/></method>
<method name="SetWallpaper"><arg type="s" direction="in"/></method>
<method name="StopWallpaper"/>
<signal name="Changed"/>
</interface></node>'''
KINDS = ('nodalix', 'system', 'apps', 'flatpak', 'firmware')
TITLES = {'nodalix': 'Nodalix', 'system': 'Programas y kernel', 'apps': 'Aplicaciones Nodalix',
          'flatpak': 'Aplicaciones Flatpak', 'firmware': 'Firmware'}
HANABI_UUID = 'hanabi-extension@jeffshee.github.io'


def program(name):
    return shutil.which(name, path=f'{Path.home()}/.local/bin:/usr/local/bin:/usr/bin')


def update_commands(kind):
    if kind in ('nodalix', 'system', 'firmware'):
        return [['/usr/bin/pkexec', '/usr/lib/nodalix/updates-privileged', kind]]
    if kind == 'apps':
        manager = program('nodalix-apps')
        if not manager:
            raise ValueError('No está instalado el gestor de aplicaciones Nodalix')
        return [[manager, 'update', '--all']]
    if kind == 'flatpak':
        if not program('flatpak'):
            raise ValueError('Flatpak no está instalado')
        return [[program('flatpak'), 'update', '--user', '-y', '--noninteractive'],
                [program('flatpak'), 'update', '--system', '-y', '--noninteractive']]
    raise ValueError('Tipo de actualización inválido')


def catalog():
    folders = [Path('/usr/share/backgrounds/nodalix/Animados'),
               Path('/usr/share/backgrounds/nodalix/animated'),
               Path.home()/'.local/share/backgrounds/nodalix/animated']
    items = []
    seen = set()
    for folder in folders:
        for video in sorted(folder.glob('*')):
            if video.suffix.lower() not in ('.mp4', '.webm', '.mkv') or not video.is_file():
                continue
            resolved = str(video.resolve())
            if resolved in seen:
                continue
            seen.add(resolved)
            name = video.stem.removeprefix('nodalix-').removesuffix('-4k').replace('-', ' ').title()
            preview = Path(os.environ.get('NODALIX_PREVIEW_DIR','/usr/share/nodalix/wallpaper-previews'))/(video.stem+'.jpg')
            items.append({'path': resolved, 'title': name, 'preview': str(preview) if preview.exists() else ''})
    return items


class SettingsService:
    def __init__(self):
        self.lock = threading.RLock()
        self.state = {'busy': False, 'checking': False, 'message': 'Pulsa Buscar actualizaciones',
                      'log': '', 'checked_at': '', 'reboot_required': False,
                      'sources': [{'kind': k, 'title': TITLES[k], 'detail': '' if self.available(k) else 'No instalado', 'available': self.available(k)} for k in KINDS]}
        self.connection = None

    def available(self, kind):
        return bool(program({'nodalix':'nodalix-updater', 'system':'pacman', 'apps':'nodalix-apps',
                             'flatpak':'flatpak', 'firmware':'fwupdmgr'}[kind]))

    def changed(self):
        if self.connection:
            GLib.idle_add(self._emit)

    def _emit(self):
        self.connection.emit_signal(None, PATH, INTERFACE, 'Changed', None)
        return GLib.SOURCE_REMOVE

    def snapshot(self):
        with self.lock:
            return json.dumps(self.state, ensure_ascii=False)

    def append_log(self, text):
        with self.lock:
            self.state['log'] = (self.state['log'] + text)[-24000:]
        self.changed()

    def start(self, target):
        if target not in KINDS and target != 'all':
            raise ValueError('Tipo de actualización inválido')
        with self.lock:
            if self.state['busy'] or self.state['checking']:
                raise ValueError('Ya hay una operación en curso')
            tasks = [k for k in KINDS if self.available(k)] if target == 'all' else [target]
            # Validate the entire plan before launching or requesting authentication.
            plans = [(k, update_commands(k)) for k in tasks]
            self.state.update(busy=True, log='', message='Preparando actualización')
        self.changed()
        threading.Thread(target=self._run, args=(plans,), daemon=True).start()

    def _run(self, plans):
        errors = []
        try:
            for kind, commands in plans:
                with self.lock:
                    self.state['message'] = 'Actualizando '+TITLES[kind]
                self.append_log('\n'+self.state['message']+'\n')
                successful = True
                for command in commands:
                    try:
                        env = {**os.environ, 'PYTHONUNBUFFERED':'1', 'TERM':'dumb'}
                        with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                              text=True, errors='replace', env=env) as process:
                            for line in process.stdout:
                                self.append_log(line)
                            code = process.wait()
                        if code:
                            raise RuntimeError(f'La operación terminó con código {code}')
                    except (OSError, RuntimeError) as error:
                        self.append_log(str(error)+'\n')
                        errors.append(TITLES[kind])
                        successful = False
                        break
                if successful and kind == 'system':
                    with self.lock:
                        self.state['reboot_required'] = True
        finally:
            with self.lock:
                self.state['busy'] = False
                self.state['message'] = ('No se completó: '+', '.join(errors)) if errors else 'Actualizaciones completadas'
            self.changed()

    def check(self):
        with self.lock:
            if self.state['busy'] or self.state['checking']:
                raise ValueError('Ya hay una operación en curso')
            self.state.update(checking=True, message='Buscando actualizaciones')
        self.changed()
        threading.Thread(target=self._check, daemon=True).start()

    def _check(self):
        try:
            for row in self.state['sources']:
                kind = row['kind']
                if not self.available(kind):
                    detail = 'No instalado'
                else:
                    try:
                        if kind == 'nodalix':
                            result = subprocess.run([program('nodalix-updater'),'check','--json'],capture_output=True,text=True,timeout=90)
                            data = json.loads(result.stdout)
                            detail = data.get('message') or data.get('status', 'Comprobado')
                            if data.get('update_available'):
                                detail = 'Nueva versión: '+str(data.get('latest_version') or data.get('version'))
                        elif kind == 'system':
                            checker = program('checkupdates')
                            result = subprocess.run([checker] if checker else [program('pacman'),'-Qu'],capture_output=True,text=True,timeout=90)
                            if result.returncode not in (0,2):
                                raise ValueError(result.stderr.strip() or 'No se pudieron consultar los repositorios')
                            count = len(result.stdout.splitlines())
                            detail = f'{count} paquetes disponibles' + ('' if checker else ' (base de datos local)')
                        elif kind == 'flatpak':
                            result = subprocess.run([program('flatpak'),'remote-ls','--updates','--columns=application'],capture_output=True,text=True,timeout=60)
                            if result.returncode:
                                raise ValueError(result.stderr.strip())
                            detail = f'{len(result.stdout.splitlines())} aplicaciones disponibles'
                        elif kind == 'firmware':
                            result = subprocess.run([program('fwupdmgr'),'get-updates','--json'],capture_output=True,text=True,timeout=45)
                            detail = 'Consulta completada' if result.returncode == 0 else 'Sin actualizaciones disponibles' if result.returncode == 2 else 'No se pudo consultar el firmware'
                        else:
                            detail = 'Actualiza las aplicaciones gestionadas por Nodalix Apps'
                    except (OSError, ValueError, subprocess.SubprocessError) as error:
                        detail = 'No se pudo comprobar: '+str(error)[:220]
                with self.lock:
                    row['detail'] = detail
                self.changed()
            with self.lock:
                self.state['checked_at'] = time.strftime('%H:%M')
                self.state['message'] = 'Comprobación terminada · '+self.state['checked_at']
        finally:
            with self.lock:
                self.state['checking'] = False
            self.changed()

    def extension(self, method):
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        result = bus.call_sync('org.gnome.Shell','/org/gnome/Shell',
                              'org.gnome.Shell.Extensions',method,GLib.Variant('(s)',(HANABI_UUID,)),
                              None,Gio.DBusCallFlags.NONE,2000,None)
        if method == 'EnableExtension' and result and not result.unpack()[0]:
            raise ValueError('El reproductor necesita volver a iniciar sesión para cargarse')

    def wallpaper(self, path):
        allowed = {item['path'] for item in catalog()}
        if path not in allowed:
            raise ValueError('El fondo no pertenece a la colección instalada')
        settings = Gio.Settings.new('io.github.jeffshee.hanabi-extension')
        settings.set_string('video-path', path)
        settings.set_boolean('mute', True)
        settings.set_boolean('show-panel-menu', False)
        settings.set_boolean('show-on-lock-screen', False)
        settings.set_int('pause-on-maximize-or-fullscreen', 1)
        settings.set_int('pause-on-battery', 2)
        self.extension('EnableExtension')
        self.changed()

    def method_call(self, connection, sender, object_path, interface, method, parameters, invocation):
        try:
            if method == 'GetUpdates':
                invocation.return_value(GLib.Variant('(s)',(self.snapshot(),)))
                return
            if method == 'GetWallpapers':
                invocation.return_value(GLib.Variant('(s)',(json.dumps(catalog(),ensure_ascii=False),)))
                return
            if method == 'CheckUpdates':
                self.check()
            elif method == 'StartUpdate':
                self.start(parameters.unpack()[0])
            elif method == 'SetWallpaper':
                self.wallpaper(parameters.unpack()[0])
            elif method == 'StopWallpaper':
                self.extension('DisableExtension')
            else:
                raise ValueError('Operación no admitida')
            invocation.return_value(None)
        except (ValueError, GLib.Error) as error:
            invocation.return_dbus_error(INTERFACE+'.Error',str(error))


def main():
    service = SettingsService()
    loop = GLib.MainLoop()
    def acquired(connection, name):
        service.connection = connection
        node = Gio.DBusNodeInfo.new_for_xml(XML)
        connection.register_object(PATH,node.interfaces[0],service.method_call,None,None)
    Gio.bus_own_name(Gio.BusType.SESSION,BUS,Gio.BusNameOwnerFlags.NONE,acquired,None,
                     lambda connection,name:loop.quit())
    loop.run()


if __name__ == '__main__':
    main()
