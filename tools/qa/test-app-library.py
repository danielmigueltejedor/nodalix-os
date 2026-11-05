import argparse
import os,sys,tempfile,subprocess,json,time
from pathlib import Path
parser = argparse.ArgumentParser(description='Test the App Library in a disposable home')
parser.add_argument('--artifacts', action='store_true', help='Write the report in the current directory')
args = parser.parse_args()
output = Path.cwd()
with tempfile.TemporaryDirectory(prefix='nodalix-app-library-qa-') as td:
 root=Path(td); home=root/'home';home.mkdir();apps=home/'Applications';apps.mkdir()
 os.environ.update(HOME=str(home),XDG_DATA_HOME=str(home/'.local/share'),XDG_STATE_HOME=str(home/'.local/state'),XDG_CONFIG_HOME=str(home/'.config'))
 repo=Path(__file__).resolve().parents[2]
 sys.path.insert(0,str(repo/'nxpm/library'));import library as lib
 fs=root/'fs';fs.mkdir()
 (fs/'usr/share/applications').mkdir(parents=True)
 (fs/'qa.desktop').symlink_to('usr/share/applications/qa.desktop')
 (fs/'usr/share/applications/qa.desktop').write_text('[Desktop Entry]\nType=Application\nName=Fixture App\nExec=DO_NOT_EXECUTE\nIcon=qa\n')
 (fs/'usr/share/icons').mkdir(parents=True)
 (fs/'usr/share/icons/qa.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="red"/></svg>')
 (fs/'.DirIcon').symlink_to('usr/share/icons/qa.svg')
 c=root/'qa.c';c.write_text('#include <stdio.h>\n#include <stdlib.h>\nint main(){char p[4096];snprintf(p,sizeof p,"%s/.qa-app-launched",getenv("HOME"));FILE*f=fopen(p,"w");if(f){fputs("launched",f);fclose(f);}return 0;}')
 exe=root/'qa';subprocess.run(['cc',str(c),'-o',str(exe)],check=True)
 sq=root/'fixture.squashfs';subprocess.run(['mksquashfs',str(fs),str(sq),'-noappend','-no-progress','-processors','1'],check=True,stdout=subprocess.DEVNULL)
 image=apps/'Fixture.AppImage';header=bytearray(exe.read_bytes());header[8:11]=b'AI\x02';header+=bytes(4096-len(header)%4096);image.write_bytes(header+sq.read_bytes())
 print('import',lib.import_appimage(image,apps))
 print((lib.DESKTOPS/'qa.desktop').read_text())
 registry={};app=lib.Gio.DesktopAppInfo.new_from_filename(str(lib.DESKTOPS/'qa.desktop'))
 print('catalog',lib.reconcile(apps,registry,[app]));print(registry)
 assert (apps/'Fixture App.app').exists() and image.is_symlink()
 assert 'DO_NOT_EXECUTE' not in (lib.DESKTOPS/'qa.desktop').read_text()
 assert app.get_icon() is not None
 lib.atomic_json(lib.STATE/'registry.json',registry)
 mime=lib.DATA/'mime/packages';mime.mkdir(parents=True)
 (mime/'nodalix-app.xml').write_text((repo/'nxpm/library/nodalix-app.xml').read_text())
 subprocess.run(['update-mime-database',str(mime.parent)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 handler=lib.DESKTOPS/'com.nodalix.Applications.desktop'
 handler.write_text('[Desktop Entry]\nType=Application\nName=Fixture Application Handler\nNoDisplay=true\nExec=/usr/bin/python '+str(repo/'nxpm/library/library.py')+' open %f\nMimeType=application/x-nodalix-app;\n')
 subprocess.run(['update-desktop-database',str(lib.DESKTOPS)],check=True)
 launch=lib.Gio.DesktopAppInfo.new_from_filename(str(handler));launch.set_as_default_for_type(lib.MIME)
 subprocess.run(['gio','open',(apps/'Fixture App.app').as_uri()],check=True)
 deadline=time.monotonic()+5
 while not (home/'.qa-app-launched').exists() and time.monotonic()<deadline:time.sleep(.02)
 assert (home/'.qa-app-launched').exists(),'Registered activation failed'
 if os.environ.get('NODALIX_NATIVE_UI_QA')=='1':
  from gi.repository import GLib,Gio
  nautilus=subprocess.Popen(['nautilus','--new-window',str(apps)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  try:
   time.sleep(3)
   assert nautilus.poll() is None,'Private file manager did not open'
   bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
   bus.call_sync('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str(output/'applications-folder.png'))),None,Gio.DBusCallFlags.NONE,5000,None)
  finally:
   nautilus.terminate();nautilus.wait(timeout=10)
 data=home/'.config/fixture/documents';data.parent.mkdir(parents=True);data.write_text('keep')
 renamed=apps/'Renamed.app';(apps/'Fixture App.app').rename(renamed)
 assert not lib.trash_requests(apps,registry),'Rename must not uninstall'
 lib.reconcile(apps,registry,[app]);lib.atomic_json(lib.STATE/'registry.json',registry)
 assert registry['qa.desktop']['filename']=='Renamed.app'
 lib.Gio.File.new_for_path(str(renamed)).trash(None)
 assert len(lib.trash_requests(apps,registry))==1,'Real GIO Trash intent not detected'
 with (lib.STATE/'lock').open('w') as lock:pass
 lib.notify=lambda text:print(text)
 lib.Library(apps).sync(import_files=False)
 assert not (lib.DESKTOPS/'qa.desktop').exists(),'Launcher was not removed'
 assert not (Path(app.get_executable())).exists(),'Payload was not removed'
 assert data.read_text()=='keep','User data changed'
 result={'real_appimage_import':True,'upstream_exec_not_executed':True,'registered_activation':True,'native_gio_trash_uninstalls':True,'rename_does_not_uninstall':True,'user_data_preserved':True,'live_user_apps_removed':False}
 if args.artifacts:
  (output/'app-library-native.json').write_text(json.dumps(result,indent=2)+'\n')
 print('Fixture installation, activation and native Trash uninstall passed')
