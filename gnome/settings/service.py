#!/usr/bin/python3
"""Session service for native Nodalix Settings pages; no compositor dependency."""
import json
import os
from pathlib import Path
import shutil
import sys
import subprocess
import threading
import time

from gi.repository import Gio, GLib
sys.path.insert(0,str(Path(__file__).resolve().parent))
from wallpaper_poster import poster_for, sync_login

BUS = 'com.nodalix.Settings'
PATH = '/com/nodalix/Settings'
INTERFACE = 'com.nodalix.Settings1'
XML = '''<node><interface name="com.nodalix.Settings1">
<method name="GetUpdates"><arg type="s" direction="out"/></method>
<method name="CheckUpdates"/>
<method name="Restart"/>
<method name="StartUpdate"><arg type="s" direction="in"/></method>
<method name="GetWallpapers"><arg type="s" direction="out"/></method>
<method name="GetWallpaperState"><arg type="s" direction="out"/></method>
<method name="SetWallpaper"><arg type="s" direction="in"/></method>
<method name="StopWallpaper"/>
<method name="GetLocalSend"><arg type="s" direction="out"/></method>
<method name="SetLocalSendOption"><arg type="s" direction="in"/><arg type="s" direction="in"/></method>
<method name="SetLocalSendEnabled"><arg type="b" direction="in"/></method>
<method name="RefreshLocalSend"/>
<method name="SetLocalSendFavorite"><arg type="s" direction="in"/><arg type="b" direction="in"/></method>
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


LOCAL_SCHEMA = 'org.gnome.shell.extensions.glocalsend'
LOCAL_OPTIONS = {'alias': str, 'download-folder': str, 'auto-accept': bool,
                 'start-on-login': bool, 'auto-disable-enabled': bool,
                 'auto-disable-minutes': int}

def localsend_settings():
    default = Gio.SettingsSchemaSource.get_default()
    schema = default.lookup(LOCAL_SCHEMA, True)
    if not schema:
        uuid = 'glocalsend@donnybeelo.github.com'
        roots = [Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share'))),
                 Path('/usr/share')]
        for root in roots:
            directory = root/'gnome-shell/extensions'/uuid/'schemas'
            if (directory/'gschemas.compiled').is_file():
                source = Gio.SettingsSchemaSource.new_from_directory(str(directory),default,False)
                schema = source.lookup(LOCAL_SCHEMA,False)
                if schema: break
    if not schema:
        raise ValueError('GLocalSend no está instalado')
    return Gio.Settings.new_full(schema,None,None)


def localsend_call(method, parameters=None):
    bus = Gio.bus_get_sync(Gio.BusType.SESSION,None)
    return bus.call_sync('com.nodalix.LocalSend','/com/nodalix/LocalSend','com.nodalix.LocalSend1',
                         method,parameters,None,Gio.DBusCallFlags.NONE,1000,None)


def localsend_snapshot():
    state={'installed':False,'ready':False,'enabled':False,'devices':[],'config':{},
           'message':'GLocalSend no está instalado'}
    try:
        prefs=localsend_settings()
        state['installed']=True
        state['config']={key:prefs.get_value(key).unpack() for key in LOCAL_OPTIONS}
        state['favorites']=prefs.get_strv('favorite-fingerprints')
        try:
            data=json.loads(localsend_call('GetStatus').unpack()[0])
            state.update(ready=True,enabled=bool(data['enabled']),devices=data['devices'])
            state['message']='Activo · '+str(len(data['devices']))+' dispositivos cercanos' if data['enabled'] else 'LocalSend desactivado'
        except GLib.Error:
            state['message']='Vuelve a iniciar sesión para conectar Ajustes y Nautilus con GLocalSend'
    except (ValueError,GLib.Error) as error:
        state['message']=str(error)
    return json.dumps(state,ensure_ascii=False)


def set_localsend_option(key,encoded):
    if key not in LOCAL_OPTIONS: raise ValueError('Opción de LocalSend no admitida')
    value=json.loads(encoded)
    if type(value) is not LOCAL_OPTIONS[key]: raise ValueError('Valor de LocalSend inválido')
    if key=='alias' and (len(value)>128 or any(ord(c)<32 for c in value)):
        raise ValueError('Nombre de dispositivo inválido')
    if key=='download-folder' and (not Path(value).is_absolute() or not Path(value).is_dir()):
        raise ValueError('Elige una carpeta de recepción existente')
    if key=='auto-disable-minutes' and not 1 <= value <= 1440:
        raise ValueError('El tiempo debe estar entre 1 y 1440 minutos')
    prefs=localsend_settings()
    # Write only the requested setting in the existing GLocalSend schema.
    if isinstance(value,bool): prefs.set_boolean(key,value)
    elif isinstance(value,int): prefs.set_int(key,value)
    else: prefs.set_string(key,value)
    Gio.Settings.sync()


def set_localsend_favorite(fingerprint,enabled):
    if not fingerprint or len(fingerprint)>256:raise ValueError('Identidad de dispositivo inválida')
    prefs=localsend_settings()
    favorites=list(prefs.get_strv('favorite-fingerprints'))
    if enabled and fingerprint not in favorites: favorites.append(fingerprint)
    if not enabled: favorites=[item for item in favorites if item!=fingerprint]
    prefs.set_strv('favorite-fingerprints',favorites)
    Gio.Settings.sync()


HANABI_SCHEMA = 'io.github.jeffshee.hanabi-extension'
RENDERER_BUS = 'io.github.jeffshee.HanabiRenderer'

def wallpaper_preferences():
    if not Gio.SettingsSchemaSource.get_default().lookup(HANABI_SCHEMA,True):
        raise ValueError('El reproductor de fondos no está instalado')
    return Gio.Settings.new(HANABI_SCHEMA)


def extension_info():
    bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
    reply=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions',
                        'GetExtensionInfo',GLib.Variant('(s)',(HANABI_UUID,)),None,
                        Gio.DBusCallFlags.NONE,1500,None)
    return reply.unpack()[0]


def request_wallpaper(enabled):
    prefs=Gio.Settings.new('org.gnome.shell')
    desired=list(prefs.get_strv('enabled-extensions'))
    if enabled and HANABI_UUID not in desired:desired.append(HANABI_UUID)
    if not enabled:desired=[uuid for uuid in desired if uuid!=HANABI_UUID]
    prefs.set_strv('enabled-extensions',desired)
    if enabled:
        prefs.set_strv('disabled-extensions',[uuid for uuid in prefs.get_strv('disabled-extensions') if uuid!=HANABI_UUID])
    Gio.Settings.sync()


def wallpaper_snapshot():
    state={'available':False,'requested':False,'active':False,'playing':False,'path':'',
           'message':'El reproductor de fondos no está instalado'}
    try:
        prefs=wallpaper_preferences();shell=Gio.Settings.new('org.gnome.shell')
        state.update(available=True,path=prefs.get_string('video-path'),
                     requested=HANABI_UUID in shell.get_strv('enabled-extensions'))
        if not state['requested']:
            state['message']='Elige una miniatura para usar un fondo animado'
        else:
            info=extension_info()
            if not info:
                state['message']='Fondo seleccionado. Cierra sesión y vuelve a entrar para activar la animación.'
            elif info.get('state') in (3,4):
                state['message']='No se pudo activar el fondo: '+str(info.get('error') or 'reproductor no compatible')
            else:
                state['message']='Activando fondo animado…'
                bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
                owner=bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus',
                                    'NameHasOwner',GLib.Variant('(s)',(RENDERER_BUS,)),None,
                                    Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                if info.get('state')==1 and owner:
                    reply=bus.call_sync(RENDERER_BUS,'/io/github/jeffshee/HanabiRenderer',
                                        'org.freedesktop.DBus.Properties','Get',
                                        GLib.Variant('(ss)',(RENDERER_BUS,'isPlaying')),None,
                                        Gio.DBusCallFlags.NONE,1000,None)
                    playing=reply.unpack()[0]
                    if isinstance(playing,GLib.Variant):playing=playing.unpack()
                    state.update(active=True,playing=bool(playing))
                    state['message']='Reproduciendo fondo animado' if playing else 'Fondo animado en pausa'
    except (ValueError,GLib.Error) as error:
        state['message']=str(error)
    return json.dumps(state,ensure_ascii=False)


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
            still = Path(os.environ.get('NODALIX_STILL_DIR','/usr/share/nodalix/wallpaper-stills'))/(video.stem+'.jpg')
            items.append({'path': resolved, 'title': name, 'preview': str(preview) if preview.exists() else '',
                          'still': str(still) if still.is_file() else ''})
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
            pending = Path('/run/nodalix-updater/reboot-required').exists()
            self.state['reboot_mandatory'] = pending
            if pending:
                self.state['reboot_required'] = True
                if not self.state['busy']:
                    self.state['message'] = 'Reinicio obligatorio para terminar la migración a GNOME'
            return json.dumps(self.state, ensure_ascii=False)

    def append_log(self, text):
        with self.lock:
            self.state['log'] = (self.state['log'] + text)[-24000:]
        self.changed()

    def start(self, target):
        if target not in KINDS and target != 'all':
            raise ValueError('Tipo de actualización inválido')
        with self.lock:
            if Path('/run/nodalix-updater/reboot-required').exists():
                raise ValueError('Reinicia ahora para terminar la migración a GNOME')
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
        items={item['path']:item for item in catalog()}
        if path not in items:
            raise ValueError('El fondo no pertenece a la colección instalada')
        preview=poster_for(Path(path))
        settings=wallpaper_preferences()
        settings.set_boolean('enable-va',True)
        settings.set_boolean('prefer-clappersink',False)
        settings.set_string('video-path',path)
        settings.set_boolean('mute',True)
        settings.set_boolean('show-panel-menu',False)
        settings.set_boolean('show-on-lock-screen',False)
        settings.set_boolean('change-wallpaper',False)
        settings.set_int('startup-delay',0)
        settings.set_int('pause-on-maximize-or-fullscreen',1)
        settings.set_int('pause-on-battery',2)
        # Native-resolution still for the desktop; thumbnails belong to the gallery.
        if preview.is_file():
            background=Gio.Settings.new('org.gnome.desktop.background')
            background.set_string('picture-uri',preview.as_uri())
            background.set_string('picture-uri-dark',preview.as_uri())
            background.set_enum('picture-options',5)  # zoom
            Gio.Settings.new('org.gnome.desktop.screensaver').set_string('picture-uri',preview.as_uri())
            # Authentication is performed asynchronously so playback never waits for it.
            def synchronize():
                try: sync_login(preview)
                except (OSError,RuntimeError,subprocess.TimeoutExpired) as error:
                    print(str(error),file=sys.stderr)
            threading.Thread(target=synchronize,daemon=True).start()
        request_wallpaper(True)
        if extension_info():self.extension('EnableExtension')
        self.changed()

    def stop_wallpaper(self):
        request_wallpaper(False)
        self.extension('DisableExtension')
        self.changed()

    def method_call(self, connection, sender, object_path, interface, method, parameters, invocation):
        try:
            if method == 'GetUpdates':
                invocation.return_value(GLib.Variant('(s)',(self.snapshot(),)))
                return
            if method == 'GetWallpapers':
                invocation.return_value(GLib.Variant('(s)',(json.dumps(catalog(),ensure_ascii=False),)))
                return
            if method == 'GetWallpaperState':
                invocation.return_value(GLib.Variant('(s)',(wallpaper_snapshot(),)))
                return
            if method == 'GetLocalSend':
                invocation.return_value(GLib.Variant('(s)',(localsend_snapshot(),)))
                return
            if method == 'SetLocalSendOption':
                set_localsend_option(*parameters.unpack())
            elif method == 'SetLocalSendFavorite':
                set_localsend_favorite(*parameters.unpack())
            elif method == 'SetLocalSendEnabled':
                desired=parameters.unpack()[0]
                result=localsend_call('SetEnabled',GLib.Variant('(b)',(desired,)))
                if bool(result.unpack()[0])!=desired: raise ValueError('No se pudo activar LocalSend; revisa su certificado')
            elif method == 'RefreshLocalSend':
                localsend_call('Refresh')
            elif method == 'CheckUpdates':
                self.check()
            elif method == 'Restart':
                if self.state['busy']:
                    raise ValueError('Espera a que termine la actualización')
                bus = Gio.bus_get_sync(Gio.BusType.SYSTEM, None)
                bus.call_sync('org.freedesktop.login1', '/org/freedesktop/login1',
                              'org.freedesktop.login1.Manager', 'Reboot',
                              GLib.Variant('(b)', (True,)), None,
                              Gio.DBusCallFlags.NONE, -1, None)
            elif method == 'StartUpdate':
                self.start(parameters.unpack()[0])
            elif method == 'SetWallpaper':
                self.wallpaper(parameters.unpack()[0])
            elif method == 'StopWallpaper':
                self.stop_wallpaper()
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
