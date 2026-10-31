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
    if marker.exists():return
    root.mkdir(parents=True,exist_ok=True,mode=0o700)
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
