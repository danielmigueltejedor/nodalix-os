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
    shortcuts_marker=root/'gnome-0.4.0-shortcuts.done'
    if not shortcuts_marker.exists():
        runpy.run_path(str(preferences))['apply_preferences']()
        from gi.repository import Gio
        settings=Gio.Settings.new('org.gnome.shell')
        uuid='nodalix-shortcuts@getnodalia.com'
        enabled=list(settings.get_strv('enabled-extensions'))
        if uuid not in enabled and uuid not in settings.get_strv('disabled-extensions'):
            settings.set_strv('enabled-extensions',enabled+[uuid])
        Gio.Settings.sync()
        shortcuts_marker.write_text('0.4.0\n')
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
