#!/usr/bin/python3
"""Test About and Phone Link widgets on a disposable real GNOME compositor."""
import argparse,json,os,shlex,subprocess,sys,tempfile,time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,default=ROOT/'dist/settings-contact-ui');p.add_argument('--control-center',type=Path);p.add_argument('--fixture',type=Path);args=p.parse_args();output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    if os.environ.get('NODALIX_NATIVE_UI_QA')!='1':
        with tempfile.TemporaryDirectory(prefix='nodalix-native-ui-') as directory:
            home=Path(directory);runtime=home/'runtime';runtime.mkdir(mode=0o700)
            env={k:v for k,v in os.environ.items() if k not in ('DBUS_SESSION_BUS_ADDRESS','DISPLAY','WAYLAND_DISPLAY','XDG_SESSION_ID','GDK_BACKEND','GNOME_SETUP_DISPLAY')}
            env.update(NODALIX_NATIVE_UI_QA='1',NODALIX_UI_HOME=directory,HOME=directory,XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(home/'data'),XDG_CACHE_HOME=str(home/'cache'),XDG_STATE_HOME=str(home/'state'),XDG_CURRENT_DESKTOP='GNOME',GTK_A11Y='none')
            result=subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--output',str(output),*(['--control-center',str(args.control_center.resolve())] if args.control_center else []),*(['--fixture',str(args.fixture.resolve())] if args.fixture else [])],env=env)
            for name in ('doc','gvfs'):subprocess.run(['fusermount3','-uz',str(runtime/name)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            return result.returncode
    from gi.repository import Gio,GLib
    home=Path(os.environ['NODALIX_UI_HOME']);qa=home/'data/gnome-shell/extensions/nodalix-ui-qa@example.invalid';qa.mkdir(parents=True)
    (qa/'metadata.json').write_text(json.dumps({'uuid':qa.name,'name':'Private UI QA','description':'Disposable compositor only','shell-version':['51']}))
    (qa/'extension.js').write_text("import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';export default class QA extends Extension{enable(){global.context.unsafe_mode=true;}disable(){global.context.unsafe_mode=false;}}")
    Gio.Settings.new('org.gnome.shell').set_strv('enabled-extensions',[qa.name])
    Gio.Settings.new('org.gnome.desktop.session').set_uint('idle-delay',0)
    Gio.Settings.new('org.gnome.desktop.screensaver').set_boolean('lock-enabled',False);Gio.Settings.sync()
    log=(output/'gnome-shell.log').open('w');process=subprocess.Popen(['gnome-shell','--headless','--wayland','--no-x11','--virtual-monitor','1280x900'],stdout=log,stderr=log)
    try:
        runtime=Path(os.environ['XDG_RUNTIME_DIR']);bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
        for _ in range(120):
            if process.poll() is not None:raise RuntimeError('Private compositor exited')
            sockets=[s for s in runtime.glob('wayland-*') if s.is_socket()]
            try:
                info=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(qa.name,)),None,Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                if sockets and info.get('state')==1:break
            except GLib.Error:pass
            time.sleep(.1)
        else:raise RuntimeError('Private compositor did not become ready')
        bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',('Main.overview.hide();true',)),None,Gio.DBusCallFlags.NONE,1000,None)
        env={**os.environ,'GDK_BACKEND':'wayland','WAYLAND_DISPLAY':sockets[0].name,'PYTHONPATH':str(ROOT/'phone-link/src')}
        if args.fixture:
            subprocess.run([sys.executable,str(args.fixture.resolve()),str(output)],check=True,env=env)
            return 0
        cc=subprocess.check_output(['pkg-config','--cflags','--libs','libadwaita-1'],text=True)
        binary=output/'test-nodalix-release'
        subprocess.run(['cc','-o',str(binary),'-I'+str(ROOT/'gnome/control-center'),str(ROOT/'tools/qa/test-nodalix-release.c'),str(ROOT/'gnome/control-center/cc-nodalix-release.c'),*shlex.split(cc)],check=True)
        subprocess.run([str(binary),str(home)],check=True,env=env)
        subprocess.run([sys.executable,str(ROOT/'tools/qa/test-contact-picker.py'),str(output)],check=True,env=env)
        if args.control_center:
            with (output/'control-center.log').open('w') as cc_log:
                app=subprocess.Popen([str(args.control_center),'system','about'],env=env,stdout=cc_log,stderr=cc_log)
                try:
                    time.sleep(3)
                    assert app.poll() is None,'Native Control Center exited'
                    bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',('Main.overview.hide();true',)),None,Gio.DBusCallFlags.NONE,1000,None)
                    time.sleep(.5)
                    bus.call_sync('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str(output/'native-about-page.png'))),None,Gio.DBusCallFlags.NONE,5000,None)
                finally:
                    app.terminate();app.wait(timeout=10)
        (output/'about-version.json').write_text(json.dumps({'initial_version':True,'atomic_update_observed':True,'invalid_identity_handled':True,'native_gtk_row':True},indent=2)+'\n')
    finally:
        process.terminate()
        try:process.wait(timeout=10)
        except subprocess.TimeoutExpired:process.kill();process.wait()
        log.close()
    return 0

if __name__=='__main__':raise SystemExit(main())
