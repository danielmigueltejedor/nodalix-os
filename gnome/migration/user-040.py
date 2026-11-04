#!/usr/bin/python3
"""Retire exact packaged extension prototypes without changing user preferences."""
import json
import os
import pwd
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
    appearance_marker=root/'gnome-0.4.0-stock-appearance.done'
    if not appearance_marker.exists():
        from gi.repository import Gio
        cleanup=preferences.with_name('desktop_040.py')
        if cleanup.is_file():
            runpy.run_path(str(cleanup))['restore_stock_appearance'](home,Gio.Settings.new('org.gnome.shell'))
            Gio.Settings.sync()
            appearance_marker.write_text('0.4.0\n')
    defaults_marker=root/'gnome-0.4.0-default-desktop-extensions.done'
    if not defaults_marker.exists():
        from gi.repository import Gio
        cleanup=preferences.with_name('desktop_040.py')
        if cleanup.is_file():
            runpy.run_path(str(cleanup))['restore_default_extensions'](home,Gio.Settings.new('org.gnome.shell'))
            Gio.Settings.sync()
            defaults_marker.write_text('0.4.0\n')
    folders_marker=root/'gnome-0.4.0-app-folders-v2.done'
    if not folders_marker.exists() and Path('/usr/bin/nodalix-organize-apps').is_file():
        adoption=runpy.run_path(str(Path(__file__).with_name('user-031.py')))
        backup=root/'gnome-0.4.0-app-folders-v2'
        config=Path(os.environ.get('XDG_CONFIG_HOME',home/'.config'))
        adoption['retire_local_units'](config,backup)
        adoption['retire_organizer_helper'](home,backup)
        subprocess.run(['systemctl','--user','daemon-reload'],check=True,timeout=15)
        subprocess.run(['systemctl','--user','try-restart','nodalix-app-organizer.path'],check=True,timeout=15)
        subprocess.run(['systemctl','--user','enable','--now','nodalix-app-organizer.path'],check=True,timeout=15)
        subprocess.run(['systemctl','--user','start','nodalix-app-organizer.service'],check=True,timeout=30)
        folders_marker.write_text('0.4.0\n')
    welcome_marker=root/'gnome-0.4.0-terminal-welcome-v2.done'
    if not welcome_marker.exists():
        terminal=runpy.run_path(str(preferences))
        config=Path(os.environ.get('XDG_CONFIG_HOME',home/'.config'))
        terminal['configure_terminal_welcome'](home,pwd.getpwuid(os.getuid()).pw_shell)
        terminal['retire_legacy_terminal_card'](config)
        welcome_marker.write_text('0.4.0\n')
    controls_marker=root/'gnome-0.4.0-window-control-icons.done'
    if not controls_marker.exists() and Path('/usr/bin/nodalix-shell-symbols').is_file():
        subprocess.run(['/usr/bin/nodalix-shell-symbols','--refresh-window-controls'],check=True,timeout=30)
        controls_marker.write_text('0.4.0\n')
    shake_marker=root/'gnome-0.4.0-shake-pointer.done'
    shake='nodalix-shake-pointer@getnodalia.com'
    if not shake_marker.exists() and (Path('/usr/share/gnome-shell/extensions')/shake/'metadata.json').is_file():
        from gi.repository import Gio
        settings=Gio.Settings.new('org.gnome.shell')
        enabled=settings.get_strv('enabled-extensions')
        if shake not in enabled and shake not in settings.get_strv('disabled-extensions'):
            settings.set_strv('enabled-extensions',enabled+[shake])
        Gio.Settings.sync()
        shake_marker.write_text('0.4.0\n')
    web_marker=root/'gnome-0.4.0-web-search.done'
    if not web_marker.exists() and Path('/usr/share/gnome-shell/search-providers/com.nodalix.WebSearch.ini').is_file():
        from gi.repository import Gio
        settings=Gio.Settings.new('org.gnome.desktop.search-providers')
        browser='org.gnome.Epiphany.desktop'
        disabled=settings.get_strv('disabled')
        if browser not in disabled and browser not in settings.get_strv('enabled'):
            settings.set_strv('disabled',disabled+[browser])
        Gio.Settings.sync()
        web_marker.write_text('0.4.0\n')
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
