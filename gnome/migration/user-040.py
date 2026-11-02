#!/usr/bin/python3
"""Retire exact packaged extension prototypes without changing user preferences."""
import json
import os
from pathlib import Path
import runpy
import subprocess


def main():
    if 'gnome' not in os.environ.get('XDG_CURRENT_DESKTOP','').lower().split(':'):return
    home=Path.home()
    root=Path(os.environ.get('XDG_STATE_HOME',home/'.local/state'))/'nodalix/migrations'
    marker=root/'gnome-0.4.0.done'
    root.mkdir(parents=True,exist_ok=True,mode=0o700)
    # This module writes only unset preferences and keeps custom launch shortcuts.
    preferences=Path(__file__).with_name('user_preferences.py')
    if not preferences.is_file():preferences=Path('/usr/lib/nodalix/user_preferences.py')
    shortcuts_marker=root/'gnome-0.4.0-native-shortcuts.done'
    if not shortcuts_marker.exists():
        from gi.repository import Gio,GLib
        import time
        settings=Gio.Settings.new('org.gnome.shell')
        uuid='nodalix-shortcuts@getnodalia.com'
        enabled=list(settings.get_strv('enabled-extensions'))
        # Release the old Shell grabs before gsd-media-keys requests the native
        # accelerators. Registering first can leave a failed grab until login.
        if uuid in enabled:
            try:
                bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
                owner=bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','NameHasOwner',GLib.Variant('(s)',('org.gnome.Shell',)),None,Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                if owner:
                    bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','DisableExtension',GLib.Variant('(s)',(uuid,)),None,Gio.DBusCallFlags.NONE,3000,None)
                    for _ in range(30):
                        info=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(uuid,)),None,Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                        if info.get('state')!=1:break
                        time.sleep(.05)
            except GLib.Error:pass
        settings.set_strv('enabled-extensions',[item for item in enabled if item!=uuid])
        settings.set_strv('disabled-extensions',[item for item in settings.get_strv('disabled-extensions') if item!=uuid])
        Gio.Settings.sync()
        runpy.run_path(str(preferences))['apply_preferences']()
        cleanup=preferences.with_name('desktop_040.py')
        if cleanup.is_file():
            module=runpy.run_path(str(cleanup))
            module['clean_extensions'](home,settings)
        close=Gio.Settings.new('org.gnome.desktop.wm.keybindings')
        bindings=list(close.get_strv('close'))
        if bindings==['<Alt>F4']:close.set_strv('close',['<Super>q','<Alt>F4'])
        Gio.Settings.sync()
        shortcuts_marker.write_text('0.4.0\n')
    tiling_marker=root/'gnome-0.4.0-tiling-assistant.done'
    if not tiling_marker.exists():
        from gi.repository import Gio
        cleanup=preferences.with_name('desktop_040.py')
        if cleanup.is_file():
            runpy.run_path(str(cleanup))['restore_tiling_assistant'](home,Gio.Settings.new('org.gnome.shell'))
            Gio.Settings.sync()
            tiling_marker.write_text('0.4.0\n')
    panels_marker=root/'gnome-0.4.0-native-panels.done'
    if not panels_marker.exists():
        from gi.repository import Gio
        cleanup=preferences.with_name('desktop_040.py')
        if cleanup.is_file():
            runpy.run_path(str(cleanup))['restore_native_panels'](home,Gio.Settings.new('org.gnome.shell'))
            prototypes=Path('/usr/share/nodalix/gnome-extension-prototypes-040.json')
            if prototypes.is_file():
                adoption=runpy.run_path(str(Path(__file__).with_name('user-031.py')))
                adoption['retire_extension_prototypes'](
                    Path(os.environ.get('XDG_DATA_HOME',home/'.local/share')),root/'gnome-0.4.0-native-panels',
                    [row for row in json.loads(prototypes.read_text()) if row['uuid']=='multi-monitors-bar@frederykabryan'])
            Gio.Settings.sync()
            panels_marker.write_text('0.4.0\n')
    if marker.exists():return
    prototypes=Path('/usr/share/nodalix/gnome-extension-prototypes-040.json')
    if prototypes.is_file():
        adoption=runpy.run_path(str(Path(__file__).with_name('user-031.py')))
        adoption['retire_extension_prototypes'](
            Path(os.environ.get('XDG_DATA_HOME',home/'.local/share')),root/'gnome-0.4.0',
            json.loads(prototypes.read_text()))
    subprocess.run(['systemctl','--user','daemon-reload'],check=True,timeout=15)
    subprocess.run(['systemctl','--user','try-restart','nodalix-user-layout.path','nodalix-user-layout.timer'],
                   check=True,timeout=30)
    marker.write_text('0.4.0\n')


if __name__=='__main__':main()
