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
    parser.add_argument('--evidence',type=Path,default=ROOT/'dist/shell-evidence')
    args=parser.parse_args();args.evidence.mkdir(parents=True,exist_ok=True)
    if os.environ.get('NODALIX_PRIVATE_SHELL_TEST')!='1':
        with tempfile.TemporaryDirectory(prefix='nodalix-private-bus-',ignore_cleanup_errors=True) as directory:
            home=Path(directory);runtime=home/'runtime';runtime.mkdir(mode=0o700)
            env={key:value for key,value in os.environ.items() if key not in ('DBUS_SESSION_BUS_ADDRESS','DISPLAY','WAYLAND_DISPLAY','XDG_SESSION_ID')}
            env.update(NODALIX_PRIVATE_SHELL_TEST='1',NODALIX_QA_HOME=directory,HOME=directory,
                XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(home/'data'),
                XDG_STATE_HOME=str(home/'state'),XDG_CACHE_HOME=str(home/'cache'))
            result=subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--evidence',str(args.evidence.resolve())],env=env)
            for name in ('doc','gvfs'):
                subprocess.run(['fusermount3','-uz',str(runtime/name)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            return result.returncode
    if '51.' not in subprocess.check_output(['gnome-shell','--version'],text=True):
        raise SystemExit('This test requires real GNOME Shell 51')
    from contextlib import nullcontext
    with nullcontext(os.environ['NODALIX_QA_HOME']) as temp:
        home=Path(temp);runtime=home/'runtime'
        data=home/'data';extensions=data/'gnome-shell/extensions';extensions.mkdir(parents=True)
        for source in (ROOT/'gnome/extensions').iterdir():
            if not source.is_dir():continue
            target=extensions/source.name;shutil.copytree(source,target)
            if (target/'schemas').is_dir():subprocess.run(['glib-compile-schemas',str(target/'schemas')],check=True)
        qa=extensions/'nodalix-qa@example.invalid';qa.mkdir()
        (qa/'metadata.json').write_text(json.dumps({'uuid':qa.name,'name':'Isolated QA','description':'Temporary test harness only','shell-version':['51']}))
        (qa/'extension.js').write_text("import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js'; export default class QA extends Extension { enable() {global.context.unsafe_mode=true;} disable() {global.context.unsafe_mode=false;} }")
        os.environ.update(HOME=str(home),XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(data),XDG_STATE_HOME=str(home/'state'),XDG_CACHE_HOME=str(home/'cache'),XDG_CURRENT_DESKTOP='GNOME')
        shell=Gio.Settings.new('org.gnome.shell')
        profile=[path.name for path in extensions.iterdir()]
        # Avoid competing with the receiver in the user's actual desktop.
        directory=extensions/'glocalsend@donnybeelo.github.com/schemas'
        source=Gio.SettingsSchemaSource.new_from_directory(str(directory),Gio.SettingsSchemaSource.get_default(),False)
        Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.glocalsend',False),None,None).set_boolean('start-on-login',False)
        shell.set_strv('enabled-extensions',profile);Gio.Settings.sync()
        with (args.evidence/'gnome-shell.log').open('w') as log:
            process=subprocess.Popen(['gnome-shell','--headless','--wayland','--no-x11','--debug-control','--virtual-monitor','1280x720','--virtual-monitor','1280x720'],stdout=log,stderr=log)
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
                before=evaluate('Main.panel.statusArea.quickSettings.menu._grid.get_children().length')
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
                (args.evidence/'lifecycle.json').write_text(json.dumps({'quick_settings_before':before,'quick_settings_after':after,'secondary_bars':bars,'enable_disable_cycles':3},indent=2))
                print('Real GNOME 51: all extensions enabled; repeated lifecycle preserved Quick Settings; two-monitor bars passed')
            finally:
                shell.set_strv('enabled-extensions',[]);Gio.Settings.sync()
                time.sleep(.3)
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
    return 0


if __name__=='__main__':raise SystemExit(main())
