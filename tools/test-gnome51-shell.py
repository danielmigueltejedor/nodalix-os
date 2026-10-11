#!/usr/bin/python3
"""Exercise extensions in a real headless GNOME 51 on a private session bus/home."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from gi.repository import Gio, GLib

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--greeter',action='store_true')
    parser.add_argument('--alternatives',action='store_true')
    parser.add_argument('--evidence',type=Path,default=ROOT/'dist/shell-evidence')
    args=parser.parse_args();args.evidence.mkdir(parents=True,exist_ok=True)
    if os.environ.get('NODALIX_PRIVATE_SHELL_TEST')!='1':
        with tempfile.TemporaryDirectory(prefix='nodalix-private-bus-',ignore_cleanup_errors=True) as directory:
            home=Path(directory);runtime=home/'runtime';runtime.mkdir(mode=0o700)
            env={key:value for key,value in os.environ.items() if key not in ('DBUS_SESSION_BUS_ADDRESS','DISPLAY','WAYLAND_DISPLAY','XDG_SESSION_ID','GDK_BACKEND','GNOME_SETUP_DISPLAY')}
            env.update(NODALIX_PRIVATE_SHELL_TEST='1',NODALIX_QA_HOME=directory,HOME=directory,
                XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(home/'data'),
                XDG_STATE_HOME=str(home/'state'),XDG_CACHE_HOME=str(home/'cache'))
            result=subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--evidence',str(args.evidence.resolve()),*(['--greeter'] if args.greeter else []),*(['--alternatives'] if args.alternatives else [])],env=env)
            for name in ('doc','gvfs'):
                subprocess.run(['fusermount3','-uz',str(runtime/name)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            return result.returncode
    if '51.' not in subprocess.check_output(['gnome-shell','--version'],text=True):
        raise SystemExit('This test requires real GNOME Shell 51')
    from contextlib import nullcontext
    with nullcontext(os.environ['NODALIX_QA_HOME']) as temp:
        home=Path(temp);runtime=home/'runtime'
        data=home/'data';extensions=data/'gnome-shell/extensions';extensions.mkdir(parents=True)
        sources=[ROOT/'gnome/settings/login-background'] if args.greeter else list((ROOT/'gnome/extensions').iterdir())
        for source in sources:
            if not source.is_dir():continue
            target=extensions/('nodalix-login-background@getnodalia.com' if args.greeter else source.name);shutil.copytree(source,target)
            metadata=target/'metadata.json'
            pinned=json.loads(metadata.read_text());pinned['version']=999999
            metadata.write_text(json.dumps(pinned))
            if (target/'schemas').is_dir():subprocess.run(['glib-compile-schemas',str(target/'schemas')],check=True)
        qa=extensions/'nodalix-qa@example.invalid';qa.mkdir()
        (qa/'metadata.json').write_text(json.dumps({'uuid':qa.name,'name':'Isolated QA','description':'Temporary test harness only','shell-version':['51'],'session-modes':['gdm'] if args.greeter else ['user']}))
        (qa/'extension.js').write_text("import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js'; export default class QA extends Extension { enable() {global.context.unsafe_mode=true;} disable() {global.context.unsafe_mode=false;} }")
        os.environ.update(HOME=str(home),XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(data),XDG_STATE_HOME=str(home/'state'),XDG_CACHE_HOME=str(home/'cache'),XDG_CURRENT_DESKTOP='GNOME')
        if not args.greeter:
            binaries=home/'bin';binaries.mkdir()
            daemon=binaries/'global-menu'
            daemon.write_text('#!/usr/bin/python3\nimport sys\nsys.path.insert(0,'+repr(str(ROOT/'gnome/lib/global-menu'))+')\nfrom globalmenu.run import main\nmain()\n')
            daemon.chmod(0o755)
            os.environ['PATH']=str(binaries)+':'+os.environ['PATH']
        shell=Gio.Settings.new('org.gnome.shell')
        profile=[path.name for path in extensions.iterdir() if path.name not in ('multi-monitors-bar@frederykabryan','tiling-assistant@leleat-on-github')]
        if args.alternatives and not args.greeter:
            profile=[uuid for uuid in profile if uuid not in ('nodalix-secondary-bars@getnodalia.com','nodalix-auto-tiling@getnodalia.com')]
            profile+=['multi-monitors-bar@frederykabryan','tiling-assistant@leleat-on-github']
        # Avoid competing with the receiver in the user's actual desktop.
        if not args.greeter:
            directory=extensions/'glocalsend@donnybeelo.github.com/schemas'
            source=Gio.SettingsSchemaSource.new_from_directory(str(directory),Gio.SettingsSchemaSource.get_default(),False)
            Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.glocalsend',False),None,None).set_boolean('start-on-login',False)
        shell.set_strv('enabled-extensions',profile);Gio.Settings.sync()
        with (args.evidence/'gnome-shell.log').open('w') as log:
            process=subprocess.Popen(['gnome-shell',*(['--mode=gdm'] if args.greeter else []),'--headless','--wayland','--no-x11','--debug-control','--virtual-monitor','1280x720','--virtual-monitor','1280x720'],stdout=log,stderr=log)
            try:
                bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
                def call(interface,method,parameters=None,path='/org/gnome/Shell'):
                    return bus.call_sync('org.gnome.Shell',path,interface,method,parameters,None,Gio.DBusCallFlags.NONE,3000,None).unpack()
                def info(uuid):
                    return call('org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(uuid,)))[0]
                deadline=time.monotonic()+60
                while time.monotonic()<deadline:
                    if process.poll() is not None:raise AssertionError('Headless GNOME exited')
                    try:
                        states={uuid:info(uuid) for uuid in profile}
                        if all(row.get('state')==1 for row in states.values()):break
                    except GLib.Error:pass
                    time.sleep(.5)
                else:raise AssertionError(states)
                (args.evidence/'extension-states.json').write_text(json.dumps(states,default=str,indent=2))
                def evaluate(code):
                    result=call('org.gnome.Shell','Eval',GLib.Variant('(s)',(code,)))
                    if not result[0]:raise AssertionError(result)
                    return json.loads(result[1])
                if args.greeter:
                    children=evaluate("Main.extensionManager.lookup('nodalix-login-background@getnodalia.com').stateObj._group.get_children().length")
                    assert children==2,children
                    print('GNOME 51 greeter: own background actors enabled on both monitors')
                    return 0
                before=evaluate('Main.panel.statusArea.quickSettings.menu._grid.get_children().length')
                if args.alternatives:
                    for uuid in ('multi-monitors-bar@frederykabryan','tiling-assistant@leleat-on-github','global-menu@unmade.space'):
                        for attempt in range(3):
                            call('org.gnome.Shell.Extensions','DisableExtension',GLib.Variant('(s)',(uuid,)))
                            time.sleep(.2)
                            call('org.gnome.Shell.Extensions','EnableExtension',GLib.Variant('(s)',(uuid,)))
                            time.sleep(.3)
                            assert info(uuid).get('state')==1,info(uuid)
                    assert evaluate('Main.panel.statusArea.quickSettings.menu._grid.get_children().length')==before
                    print('Real GNOME 51: alternative multi-monitor bars, Tiling Assistant and Global Menu lifecycle passed')
                    return 0
                for uuid in ('glocalsend@donnybeelo.github.com','nodalix-connect@getnodalia.com','nodalix-secondary-bars@getnodalia.com'):
                    for attempt in range(3):
                        call('org.gnome.Shell.Extensions','DisableExtension',GLib.Variant('(s)',(uuid,)))
                        time.sleep(.2)
                        call('org.gnome.Shell.Extensions','EnableExtension',GLib.Variant('(s)',(uuid,)))
                        time.sleep(.3)
                        assert info(uuid).get('state')==1,info(uuid)
                after=evaluate('Main.panel.statusArea.quickSettings.menu._grid.get_children().length')
                assert after==before,(before,after)
                bars=evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")
                assert bars==1,bars
                workareas=evaluate('Main.layoutManager.monitors.map(m => ({monitor:[m.x,m.y,m.width,m.height],work:[global.workspace_manager.get_active_workspace().get_work_area_for_monitor(m.index).x,global.workspace_manager.get_active_workspace().get_work_area_for_monitor(m.index).y,global.workspace_manager.get_active_workspace().get_work_area_for_monitor(m.index).width,global.workspace_manager.get_active_workspace().get_work_area_for_monitor(m.index).height]}))')
                assert all(row['work'][1]>row['monitor'][1] for row in workareas),workareas
                def display_call(method,params=None):
                    return bus.call_sync('org.gnome.Mutter.DisplayConfig','/org/gnome/Mutter/DisplayConfig','org.gnome.Mutter.DisplayConfig',method,params,None,Gio.DBusCallFlags.NONE,5000,None).unpack()
                current=display_call('GetCurrentState')
                modes={monitor[0][0]:next(mode[0] for mode in monitor[1] if mode[6].get('is-current')) for monitor in current[1]}
                original=[(row[0],row[1],row[2],row[3],row[4],[(spec[0],modes[spec[0]],{}) for spec in row[5]]) for row in current[2]]
                def configure(layout):
                    serial=display_call('GetCurrentState')[0]
                    display_call('ApplyMonitorsConfig',GLib.Variant('(uua(iiduba(ssa{sv}))a{sv})',(serial,1,layout,{})))
                    time.sleep(1)
                primary=next(row for row in original if row[4])
                configure([(0,0,primary[2],primary[3],True,primary[5])])
                assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==0
                configure(original)
                assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==1
                scale_test=False
                if all(2.0 in mode[5] for monitor in current[1] for mode in monitor[1] if mode[6].get('is-current')):
                    scaled=[];offset=0
                    for row in original:
                        scaled.append((offset,0,2.0,row[3],row[4],row[5]));offset+=640
                    configure(scaled)
                    assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==1
                    configure(original);scale_test=True
                fixture="""import gi
gi.require_version('Gtk','4.0')
from gi.repository import Gtk
app=Gtk.Application(application_id='com.nodalix.FullscreenQA')
def activate(app):
    window=Gtk.ApplicationWindow(application=app,title='Nodalix fullscreen QA',default_width=500,default_height=300)
    window.present()
app.connect('activate',activate);app.run()
"""
                env={**os.environ,'GDK_BACKEND':'wayland','WAYLAND_DISPLAY':next(path.name for path in runtime.glob('wayland-*') if path.is_socket())}
                app=subprocess.Popen([sys.executable,'-c',fixture],env=env,stdout=log,stderr=log)
                try:
                    for attempt in range(30):
                        if evaluate("global.get_window_actors().some(a => a.meta_window.get_title() === 'Nodalix fullscreen QA')"):break
                        time.sleep(.1)
                    else:raise AssertionError('GTK fullscreen fixture did not map')
                    evaluate("Main.overview.hide(); true")
                    time.sleep(.5)
                    evaluate("(() => { const w=global.get_window_actors().find(a=>a.meta_window.get_title()==='Nodalix fullscreen QA').meta_window; w.move_to_monitor(Main.layoutManager.monitors.find(m=>m.index!==Main.layoutManager.primaryIndex).index); w.make_fullscreen(); return true; })()")
                    time.sleep(.8)
                    assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.every(b=>!b.bar.visible)")
                    evaluate("global.get_window_actors().find(a=>a.meta_window.get_title()==='Nodalix fullscreen QA').meta_window.unmake_fullscreen(); true")
                    time.sleep(.8)
                    assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.every(b=>b.bar.visible)")
                finally:
                    app.terminate();app.wait(timeout=10)
                (args.evidence/'lifecycle.json').write_text(json.dumps({'quick_settings_before':before,'quick_settings_after':after,'secondary_bars':bars,'enable_disable_cycles':3,'workareas':workareas,'monitor_reconfiguration':True,'fullscreen_hide_restore':True,'scale_2_supported_and_tested':scale_test},indent=2))
                print('Real GNOME 51: all extensions enabled; repeated lifecycle preserved Quick Settings; two-monitor bars passed')
            finally:
                shell.set_strv('enabled-extensions',[]);Gio.Settings.sync()
                time.sleep(.3)
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
    return 0


if __name__=='__main__':raise SystemExit(main())
