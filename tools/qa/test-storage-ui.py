"""Exercise the production native Settings widgets with disposable files and apps."""
from pathlib import Path
import json
import os
import subprocess
import sys
from tempfile import TemporaryDirectory
import time

import shlex
from gi.repository import Gio, GLib

home = Path.home()
output = Path(sys.argv[1])
repo = Path(__file__).resolve().parents[2]
config = Path(os.environ['XDG_CONFIG_HOME']); config.mkdir(parents=True, exist_ok=True)
data = Path(os.environ['XDG_DATA_HOME']); state = Path(os.environ['XDG_STATE_HOME'])
folders = {name: home/path for name, path in [('DOCUMENTS','Documents'),('MUSIC','Music'),('DOWNLOAD','Downloads')]}
for folder in folders.values():folder.mkdir(parents=True, exist_ok=True)
(config/'user-dirs.dirs').write_text(''.join(f'XDG_{name}_DIR="{path}"\n' for name,path in folders.items()))

def sized(path, length):
    with path.open('wb') as file:file.truncate(length)

large=folders['DOCUMENTS']/'Large test document.pdf'; small=folders['DOCUMENTS']/'Small test document.pdf'
sized(large,34*1024**2);sized(small,128);sized(folders['MUSIC']/'Test track.flac',2*1024**2)
cache=home/'.cache/pip';cache.mkdir(parents=True);sized(cache/'fixture-cache',3*1024**2)
apps=home/'Applications';apps.mkdir();desktops=data/'applications';desktops.mkdir(parents=True,exist_ok=True)
registry={}
for name,size in [('Large',8*1024**2),('Small',1024)]:
    ident=f'org.example.{name}.desktop'; payload=home/(name+'.AppImage');sized(payload,size);payload.chmod(0o700)
    (desktops/ident).write_text(f'[Desktop Entry]\nType=Application\nName={name} fixture app\nExec="{payload}"\nIcon=application-x-executable\n')
    filename=f'{name} fixture app.app';token=f'synthetic-{name}'
    (apps/filename).write_text(json.dumps({'format':'nodalix-app-1','desktop_id':ident,'token':token}))
    registry[ident]={'filename':filename,'token':token,'name':name}
record=state/'nodalix/applications/registry.json';record.parent.mkdir(parents=True);record.write_text(json.dumps(registry))
helper='/usr/lib/nodalix/storage-manager'
snapshot=json.loads(subprocess.check_output([helper,'scan'],text=True))
assert [x['name'] for x in snapshot['categories']['apps']]==['Large fixture app','Small fixture app']
assert [x['name'] for x in snapshot['categories']['documents']]==[large.name]
assert snapshot['categories']['music'] and snapshot['categories']['development']
with TemporaryDirectory(prefix="nodalix-storage-test-") as scratch:
    binary=Path(scratch)/'storage-fixture'
    flags=shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','libadwaita-1','json-glib-1.0'],text=True))
    subprocess.run(['cc','-o',str(binary),'-I'+str(repo/'gnome/control-center'),str(repo/'tools/qa/test-storage-page.c'),*flags],check=True)
    ready=home/'storage-ready'
    with (output/'storage-fixture.log').open('w') as log:
        process=subprocess.Popen([str(binary)],stdout=log,stderr=log,env={**os.environ,'STORAGE_UI_READY':str(ready)})
        try:
            deadline=time.monotonic()+10
            while not ready.exists() and time.monotonic()<deadline:
                assert process.poll() is None,'Compiled Storage page exited'
                time.sleep(.1)
            assert ready.exists(),'Storage page did not render fixture sizes'
            bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
            bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',('Main.overview.hide();true',)),None,Gio.DBusCallFlags.NONE,5000,None)
            time.sleep(.6)
            bus.call_sync('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str(output/'storage-page.png'))),None,Gio.DBusCallFlags.NONE,5000,None)
        finally:
            process.terminate();process.wait(timeout=10)
subprocess.run([helper,'trash-file',str(large)],check=True)
assert not large.exists() and small.exists()
subprocess.run([helper,'trash-app','org.example.Small.desktop'],check=True)
assert not (apps/'Small fixture app.app').exists() and (apps/'Large fixture app.app').exists()
assert (home/'Small.AppImage').exists(), 'Only the Applications service should process the uninstall'
(output/'storage-native.json').write_text(json.dumps({'production_storage_widgets_rendered':True,'app_sizes_sorted':True,'large_documents_filtered':True,'music_and_development_categories':True,'selected_document_trashed':True,'registered_app_trash_intent':True,'real_user_files_removed':False},indent=2)+'\n')
print('Native Storage page, ordered apps, large documents and both Trash routes passed')
