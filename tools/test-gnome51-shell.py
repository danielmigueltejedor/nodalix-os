#!/usr/bin/python3
"""Exercise extensions in a real headless GNOME 51 on a private session bus/home."""
import argparse
import importlib.util
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
    parser.add_argument('--without-phone',action='store_true')
    parser.add_argument('--panels-only',action='store_true')
    parser.add_argument('--evidence',type=Path,default=ROOT/'dist/shell-evidence')
    args=parser.parse_args();args.evidence.mkdir(parents=True,exist_ok=True)
    if os.environ.get('NODALIX_PRIVATE_SHELL_TEST')!='1':
        with tempfile.TemporaryDirectory(prefix='nodalix-private-bus-',ignore_cleanup_errors=True) as directory:
            home=Path(directory);runtime=home/'runtime';runtime.mkdir(mode=0o700)
            env={key:value for key,value in os.environ.items() if key not in ('DBUS_SESSION_BUS_ADDRESS','DISPLAY','WAYLAND_DISPLAY','XDG_SESSION_ID','GDK_BACKEND','GNOME_SETUP_DISPLAY')}
            env.update(NODALIX_PRIVATE_SHELL_TEST='1',NODALIX_QA_HOME=directory,HOME=directory,GTK_A11Y='none',
                XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'config'),XDG_DATA_HOME=str(home/'data'),
                XDG_STATE_HOME=str(home/'state'),XDG_CACHE_HOME=str(home/'cache'))
            result=subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--evidence',str(args.evidence.resolve()),*(['--greeter'] if args.greeter else []),*(['--alternatives'] if args.alternatives else []),*(['--without-phone'] if args.without_phone else []),*(['--panels-only'] if args.panels_only else [])],env=env)
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
        schemas=home/'schemas';schemas.mkdir()
        for source in Path('/usr/share/glib-2.0/schemas').glob('*.xml'):
            (schemas/source.name).symlink_to(source)
        shutil.copy2(ROOT/'gnome/session/90-nodalix.gschema.override',schemas/'90-nodalix.gschema.override')
        subprocess.run(['glib-compile-schemas','--strict',str(schemas)],check=True)
        os.environ['GSETTINGS_SCHEMA_DIR']=str(schemas)
        if not args.greeter and not args.alternatives:
            for row in json.loads((ROOT/'gnome/session/extension-defaults.json').read_text()):
                local=Gio.SettingsSchemaSource.new_from_directory(str(extensions/row['uuid']/'schemas'),Gio.SettingsSchemaSource.get_default(),False)
                prefs=Gio.Settings.new_full(local.lookup(row['schema'],False),None,None)
                for key,value in row['values'].items():prefs.set_value(key,GLib.Variant.parse(None,value,None,None))
                if row['uuid']=='dash-to-dock@micxgx.gmail.com':dock=prefs
        if not args.greeter:
            binaries=home/'bin';binaries.mkdir()
            daemon=binaries/'global-menu'
            daemon.write_text('#!/usr/bin/python3\nimport sys\nsys.path.insert(0,'+repr(str(ROOT/'gnome/lib/global-menu'))+')\nfrom globalmenu.run import main\nmain()\n')
            daemon.chmod(0o755)
            os.environ['PATH']=str(binaries)+':'+os.environ['PATH']
        shell=Gio.Settings.new('org.gnome.shell')
        # Only the disposable session: a host suspend must not lock its QA windows.
        Gio.Settings.new('org.gnome.desktop.screensaver').set_boolean('lock-enabled',False)
        Gio.Settings.new('org.gnome.desktop.session').set_uint('idle-delay',0)
        Gio.Settings.new('org.gnome.desktop.interface').set_boolean('enable-hot-corners',False)
        Gio.Settings.new('org.gnome.mutter').set_string('overlay-key','Super_L')
        Gio.Settings.new('org.gnome.desktop.input-sources').set_value('sources',GLib.Variant('a(ss)',[('xkb','es')]))
        profile=[path.name for path in extensions.iterdir() if path.name!='multi-monitors-bar@frederykabryan']
        if args.without_phone:profile=[uuid for uuid in profile if uuid!='nodalix-connect@getnodalia.com']
        if args.alternatives and not args.greeter:
            profile=[uuid for uuid in profile if uuid not in ('nodalix-secondary-bars@getnodalia.com','nodalix-auto-tiling@getnodalia.com')]
            profile+=['multi-monitors-bar@frederykabryan']
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
                if not args.alternatives:
                    # Inspect the very first desktop appearance, before any
                    # hide/show cycle can repair a stale startup blur clip.
                    for _ in range(160):
                        if evaluate('!Main.layoutManager._startingUp'):break
                        time.sleep(.05)
                    else:raise AssertionError('Native startup animation did not finish')
                    evaluate('Main.overview.hide();true')
                    for _ in range(100):
                        if evaluate('global.blur_my_shell._dash_to_dock_blur.dashes.every(d=>d.dash.opacity===255&&d.dash.translation_x===0&&d.dash.translation_y===0&&d.dash_container._slider.slide_x===1)'):break
                        time.sleep(.05)
                    else:raise AssertionError('Native dock did not finish its initial appearance')
                    time.sleep(.6)  # Native icon-size transitions finish after the slide.
                    cold_dock=evaluate("global.blur_my_shell._dash_to_dock_blur.dashes.map(d=>({width:d.dash_background.width,height:d.dash_background.height,clip:d.background.has_clip?d.background.get_clip():null,origin:d.background.get_transformed_position(),target:d.dash_background.get_transformed_position(),static:d.dash_blur.is_static,configured:d.settings.dash_to_dock.STATIC_BLUR,container:[d.dash_container.x,d.dash_container.y,d.dash_container.width,d.dash_container.height,d.dash_container.translation_x,d.dash_container.translation_y],tracked:!!d.dash_background_allocation_id}))")
                    assert len(cold_dock)==2,cold_dock
                    assert all(d['tracked'] and d['clip'] and d['width']>0 and d['height']>0 and d['clip'][2]>=d['width']-.5 and d['clip'][3]>=d['height']-.5 for d in cold_dock),cold_dock
                    assert all(abs(d['origin'][1]+d['clip'][1]-d['target'][1]+1)<1 for d in cold_dock),cold_dock
                    (args.evidence/'dock-first-frame.json').write_text(json.dumps(cold_dock,indent=2))
                    evaluate("(() => {const d=global.blur_my_shell._dash_to_dock_blur.dashes[0];global._qaDockBackgroundBox=d.dash_background.get_allocation_box().copy();const b=global._qaDockBackgroundBox.copy();b.x2=b.x1+b.get_width()/2;d.dash_background.allocate(b);return true;})()")
                    time.sleep(.15)
                    resized=evaluate("(() => {const d=global.blur_my_shell._dash_to_dock_blur.dashes[0];return {width:d.dash_background.width,clip:d.background.get_clip()};})()")
                    assert resized['width']<cold_dock[0]['width']*.75,resized
                    assert abs(resized['clip'][2]-resized['width']-2)<1,resized
                    evaluate('global.blur_my_shell._dash_to_dock_blur.dashes[0].dash_background.allocate(global._qaDockBackgroundBox);delete global._qaDockBackgroundBox;true')
                    time.sleep(.15)
                    restored=evaluate('global.blur_my_shell._dash_to_dock_blur.dashes[0].background.get_clip()[2]')
                    assert abs(restored-cold_dock[0]['clip'][2])<1,restored
                    (args.evidence/'dock-background-allocation.json').write_text(json.dumps({'initial':cold_dock[0],'independent_background_allocation':resized,'restored_clip_width':restored},indent=2))
                    evaluate('global.blur_my_shell._dash_to_dock_blur.dashes[0].dash.translation_y=30;true');time.sleep(.15)
                    translated=evaluate('(() => {const d=global.blur_my_shell._dash_to_dock_blur.dashes[0];return {clip:d.background.get_clip(),origin:d.background.get_transformed_position(),target:d.dash_background.get_transformed_position()};})()')
                    assert abs(translated['origin'][1]+translated['clip'][1]-translated['target'][1]+1)<1,translated
                    evaluate('global.blur_my_shell._dash_to_dock_blur.dashes[0].dash.translation_y=0;true');time.sleep(.15)
                    (args.evidence/'dock-startup-translation.json').write_text(json.dumps(translated,indent=2))
                    secondary="Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj"
                    roles=evaluate(f'[...{secondary}._bars[0].buttons.keys()]')
                    assert all(role in roles for role in ('activities','dateMenu','quickSettings')),roles
                    blur=evaluate(f'global.blur_my_shell._panel_blur.actors_list.filter(a=>a.widgets.panel===Main.panel||a.widgets.panel==={secondary}._bars[0].panel).map(a=>({{monitor:a.monitor.index,static:a.static_blur,background:!!a.widgets.background}}))')
                    assert len(blur)==2 and all(row['static'] and row['background'] for row in blur),blur
                    menus=[]
                    for role in ('quickSettings','dateMenu'):
                        evaluate(f"{secondary}._bars[0].buttons.get('{role}').emit('clicked',1);true")
                        time.sleep(.5)
                        geometry=evaluate(f"(() => {{const menu=Main.panel.statusArea.{role}.menu,b={secondary}._bars[0].buttons.get('{role}'),m=Main.layoutManager.findMonitorForActor(b),p=menu._boxPointer.get_transformed_position(),s=menu._boxPointer.get_transformed_size();return {{open:menu.isOpen,sameSource:menu.sourceActor===b,monitor:m.index,x:p[0],y:p[1],width:s[0],height:s[1],bounds:[m.x,m.y,m.width,m.height]}};}})()")
                        assert geometry['open'] and geometry['sameSource'],geometry
                        x,y,w,h=geometry['bounds']
                        assert x-.5<=geometry['x'] and geometry['x']+geometry['width']<=x+w+.5,geometry
                        assert y<=geometry['y'] and geometry['y']+geometry['height']<=y+h+.5,geometry
                        if role=='quickSettings':
                            bus.call_sync('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str((args.evidence/'secondary-panel.png').resolve()))),None,Gio.DBusCallFlags.NONE,5000,None)
                        evaluate(f'Main.panel.statusArea.{role}.menu.close({{animate:false}});true');time.sleep(.1)
                        assert evaluate(f'Main.panel.statusArea.{role}.menu.sourceActor===Main.panel.statusArea.{role}'),role
                        menus.append({'role':role,**geometry})
                    assert evaluate('Main.panel.statusArea.quickSettings.menu._grid.get_children().length')==before
                    panel_geometry=evaluate(f"(() => {{const e={secondary}._bars[0];return {{primary:Main.panel.height,primaryBox:Main.layoutManager.panelBox.height,secondary:e.panel.height,secondaryBox:e.bar.height,buttons:[...e.buttons.entries()].map(([role,b])=>({{role,height:b.height,content:b.get_first_child().height,source:Main.panel.statusArea[role].height}}))}};}})()")
                    (args.evidence/'secondary-native-menus.json').write_text(json.dumps({'roles':roles,'blur':blur,'menus':menus,'panel_geometry':panel_geometry,'primary_quick_settings_unchanged':True},indent=2))
                    if args.panels_only:
                        print('Native secondary menus and first-appearance dock geometry passed')
                        return 0
                # Real freedesktop notifications, in this private bus only.
                def notification(app, title, hints=None, actions=None, replace=0):
                    return bus.call_sync('org.freedesktop.Notifications', '/org/freedesktop/Notifications',
                        'org.freedesktop.Notifications', 'Notify',
                        GLib.Variant('(susssasa{sv}i)', (app, replace, 'phone-symbolic', title,
                            'Isolated Nodalix QA', actions or [], hints or {}, 0)),
                        None, Gio.DBusCallFlags.NONE, 5000, None).unpack()[0]
                if not args.alternatives:
                    call_id=notification('Nodalix QA call', 'Test call',
                        {'x-nodalix-phone-call':GLib.Variant('b',True), 'resident':GLib.Variant('b',True),
                         'urgency':GLib.Variant('y',2)}, ['answer','Answer','decline','Decline'])
                    for index in range(18):
                        notification('Nodalix QA '+str(index), 'Notification '+str(index),
                                     {'urgency':GLib.Variant('y',2 if index==17 else 1)})
                    time.sleep(.8)
                    view='Main.panel.statusArea.dateMenu._messageList._messageView'
                    assert evaluate(f'{view}.messages[0]._notificationToMessage.has(Main.notificationDaemon._fdoNotificationDaemon._notifications.get({call_id}))')
                    assert evaluate(f'!{view}.messages[0].canClose()')
                    assert evaluate(f'![...{view}.messages[0]._notificationToMessage.values()][0]._header.closeButton.visible')
                    evaluate('Main.overview.hide(); Main.panel.statusArea.dateMenu.menu.open(); true')
                    time.sleep(.5)
                    geometry=evaluate("(() => {const d=Main.panel.statusArea.dateMenu,l=d._messageList,s=l._scrollView,m=Main.layoutManager.primaryMonitor; return {panelHeight:d.menu.actor.height,height:s.height,content:s.vadjustment.upper,page:s.vadjustment.page_size,monitor:m.height,controls:l._clearButton.get_transformed_position()[1]+l._clearButton.height};})()")
                    assert 0<geometry['height']<geometry['monitor']*.8,geometry
                    assert geometry['content']>geometry['page'],geometry
                    assert geometry['panelHeight']<=geometry['monitor']*.95,geometry
                    assert geometry['controls']<=geometry['monitor'],geometry
                    evaluate(f'[...{view}.messages.at(-1)._notificationToMessage.values()][0].grab_key_focus(); true')
                    time.sleep(.3)
                    assert evaluate('Main.panel.statusArea.dateMenu._messageList._scrollView.vadjustment.value')>0
                    evaluate(f'{view}.clear(); true')
                    time.sleep(1)
                    assert evaluate(f'Main.notificationDaemon._fdoNotificationDaemon._notifications.has({call_id})')
                    remaining=evaluate('Main.notificationDaemon._fdoNotificationDaemon._notifications.size')
                    assert remaining==1,remaining
                    received=[]
                    sub=bus.signal_subscribe(None,'org.freedesktop.Notifications','ActionInvoked',
                        '/org/freedesktop/Notifications',None,Gio.DBusSignalFlags.NONE,
                        lambda *args:received.append(args[5].unpack()))
                    evaluate(f'Main.notificationDaemon._fdoNotificationDaemon._notifications.get({call_id}).actions[0].activate(); true')
                    deadline=time.monotonic()+2
                    while time.monotonic()<deadline and not received:
                        while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
                        time.sleep(.02)
                    assert (call_id,'answer') in received,received
                    evaluate(f'Main.notificationDaemon._fdoNotificationDaemon._notifications.get({call_id}).actions[1].activate(); true')
                    notification('Nodalix QA call','Active call',
                        {'x-nodalix-phone-call':GLib.Variant('b',True),'resident':GLib.Variant('b',True)},
                        ['hangup','Hang up'],replace=call_id)
                    evaluate(f'Main.notificationDaemon._fdoNotificationDaemon._notifications.get({call_id}).actions[0].activate(); true')
                    deadline=time.monotonic()+2
                    while time.monotonic()<deadline and (call_id,'hangup') not in received:
                        while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
                        time.sleep(.02)
                    bus.signal_unsubscribe(sub)
                    assert all((call_id,action) in received for action in ('answer','decline','hangup')),received
                    bus.call_sync('org.freedesktop.Notifications','/org/freedesktop/Notifications',
                        'org.freedesktop.Notifications','CloseNotification',GLib.Variant('(u)',(call_id,)),
                        None,Gio.DBusCallFlags.NONE,3000,None)
                    time.sleep(.5)
                    assert evaluate(f'!Main.notificationDaemon._fdoNotificationDaemon._notifications.has({call_id})')
                    assert evaluate('Main.panel.statusArea.dateMenu._messageList._scrollView.height')<geometry['height']
                    evaluate('Main.panel.statusArea.dateMenu.menu.close(); true')
                    (args.evidence/'notifications.json').write_text(json.dumps({
                        'live_call_first_after_18_notifications':True,'critical_notification_below_call':True,
                        'clear_preserves_call_only':True,'close_control_hidden':True,
                        'answer_decline_hangup_signals':True,'replacement_and_end':True,'empty_list_shrinks':True,
                        'scroll_geometry':geometry,'keyboard_scroll_to_last':True},indent=2))
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
                    if uuid not in profile:continue
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
                for _ in range(100):
                    if evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==1:break
                    time.sleep(.05)
                assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==1
                scale_test=False
                if all(2.0 in mode[5] for monitor in current[1] for mode in monitor[1] if mode[6].get('is-current')):
                    scaled=[];offset=0
                    for row in original:
                        scaled.append((offset,0,2.0,row[3],row[4],row[5]));offset+=640
                    configure(scaled)
                    assert evaluate("Main.extensionManager.lookup('nodalix-secondary-bars@getnodalia.com').stateObj._bars.length")==1
                    configure(original);scale_test=True
                evaluate("import(Main.extensionManager.lookup('dash-to-dock@micxgx.gmail.com').dir.get_uri()+'/extension.js').then(m=>global._qaDockManager=m.dockManager);true")
                time.sleep(.3)
                evaluate('Main.overview.hide();global._qaPointer=global.stage.get_context().get_backend().get_default_seat().create_virtual_device(imports.gi.Clutter.InputDeviceType.POINTER_DEVICE);true')
                dock.set_boolean('intellihide',False);Gio.Settings.sync()
                dock_results=[]
                for monitor in range(2):
                    # Force autohide in the empty private desktop. Moving
                    # rapidly to the edge must still reveal the hidden dock.
                    evaluate(f"(() => {{let m=Main.layoutManager.monitors[{monitor}],d=global._qaPointer,t=imports.gi.GLib.get_monotonic_time();d.notify_absolute_motion(t,m.x+m.width/2,m.y+m.height/2);return true;}})()")
                    time.sleep(.7)
                    assert evaluate(f"global._qaDockManager._allDocks.find(d=>d.monitorIndex==={monitor}).dockState")==0
                    evaluate(f"(() => {{let m=Main.layoutManager.monitors[{monitor}],d=global._qaPointer,t=imports.gi.GLib.get_monotonic_time();d.notify_absolute_motion(t,m.x+m.width/2,m.y+m.height-20);d.notify_absolute_motion(t+1000,m.x+m.width/2,m.y+m.height-1);return true;}})()")
                    time.sleep(.7)
                    state=evaluate(f"(() => {{let d=global._qaDockManager._allDocks.find(d=>d.monitorIndex==={monitor});return {{state:d.dockState,slide:d._slider.slide_x,mapped:d.mapped}};}})()")
                    assert state['state']==2 and state['slide']>.9,state
                    dock_results.append(state)
                (args.evidence/'dock-reveal.json').write_text(json.dumps(dock_results,indent=2))
                dock.set_boolean('intellihide',True);Gio.Settings.sync()
                evaluate('(() => {let m=Main.layoutManager.primaryMonitor;global._qaPointer.notify_absolute_motion(imports.gi.GLib.get_monotonic_time(),m.x+m.width/2,m.y+m.height/2);return true;})()')
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
                # A real GTK text entry receives physical-style Super key events.
                entry_fixture="""import gi
gi.require_version('Gtk','4.0')
from gi.repository import Gtk,Gio
app=Gtk.Application(application_id='com.nodalix.EntryQA')
def activate(app):
    win=Gtk.ApplicationWindow(application=app,title='Nodalix Entry QA',default_width=400,default_height=200)
    entry=Gtk.Entry();win.set_child(entry)
    entry.connect('changed',lambda *args:win.set_title('Nodalix Entry QA: '+entry.get_text()))
    def prepare(action,param):
        entry.set_text('Nodalix copy QA' if action.get_name()=='prepare-copy' else '')
        entry.grab_focus();entry.select_region(0,-1);win.present()
    for name in ('prepare-copy','prepare-paste'):
        action=Gio.SimpleAction.new(name,None);action.connect('activate',prepare);app.add_action(action)
    win.present();entry.grab_focus()
app.connect('activate',activate);app.run()
"""
                # Native GNOME custom shortcuts, with a fixture command instead
                # of the privileged kernel broker in this private compositor.
                command=home/'clipboard-command.py'
                command.write_text("""import sys,time
from gi.repository import Gio,GLib
mod=29 if sys.argv[1]=='copy' else 42
code=f'(() => {{ const d=global._qaKeyboard;let t=imports.gi.GLib.get_monotonic_time();d.notify_key(t+=10,{mod},1);d.notify_key(t+=10,110,1);d.notify_key(t+=10,110,0);d.notify_key(t+=10,{mod},0);return true; }})()'
time.sleep(.1)
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
result=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',(code,)),None,Gio.DBusCallFlags.NONE,3000,None).unpack()
assert result[0],result
""")
                media=Gio.Settings.new('org.gnome.settings-daemon.plugins.media-keys')
                paths=[]
                for action,key in [('copy','c'),('paste','v')]:
                    path='/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/nodalix-'+action+'/'
                    setting=Gio.Settings.new_with_path('org.gnome.settings-daemon.plugins.media-keys.custom-keybinding',path)
                    setting.set_string('name','Nodalix '+action)
                    setting.set_string('command',sys.executable+' '+str(command)+' '+action)
                    setting.set_string('binding','<Super>'+key);paths.append(path)
                media.set_strv('custom-keybindings',paths);Gio.Settings.sync()
                media_keys=subprocess.Popen(['/usr/lib/gsd-media-keys'],env=env,stdout=log,stderr=log)
                time.sleep(1)
                entry=subprocess.Popen([sys.executable,'-c',entry_fixture],env=env,stdout=log,stderr=log)
                def action(name):
                    bus.call_sync('com.nodalix.EntryQA','/com/nodalix/EntryQA','org.gtk.Actions','Activate',
                        GLib.Variant('(sava{sv})',(name,[],{})),None,Gio.DBusCallFlags.NONE,2000,None)
                def keys(key,modifiers=(0xffeb,)):
                    codes={ord('a'):30,ord('c'):46,ord('v'):47,ord('q'):16,ord('3'):4,ord('4'):5,0xff0d:28,0xff08:14}
                    mods=[{0xffeb:125,0xffe1:42,0xffe3:29}[value] for value in modifiers]
                    evaluate(f"(() => {{ const d=global._qaKeyboard;let t=imports.gi.GLib.get_monotonic_time();for(const k of {json.dumps(mods)})d.notify_key(t+=10,k,1);d.notify_key(t+=10,{codes[key]},1);d.notify_key(t+=10,{codes[key]},0);for(const k of {json.dumps(list(reversed(mods)))})d.notify_key(t+=10,k,0);return true;}})()")
                try:
                    evaluate('global._qaKeyboard=global.stage.get_context().get_backend().get_default_seat().create_virtual_device(imports.gi.Clutter.InputDeviceType.KEYBOARD_DEVICE); true')
                    for attempt in range(60):
                        if evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()?.startsWith('Nodalix Entry QA'))"):break
                        time.sleep(.05)
                    else:raise AssertionError('GTK clipboard fixture did not map')
                    evaluate('Main.overview.hide(); true')
                    action('prepare-copy');time.sleep(.3);keys(ord('c'));time.sleep(.3)
                    action('prepare-paste');time.sleep(.3);keys(ord('v'));time.sleep(.4)
                    assert evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()==='Nodalix Entry QA: Nodalix copy QA')")
                    focus=evaluate("new imports.gi.Gio.Settings({schema_id:'org.gnome.desktop.wm.preferences'}).get_string('focus-mode')")
                    assert focus=='sloppy',focus
                    terminal_program=home/'terminal-fixture.py'
                    terminal_program.write_text("import signal,sys\nfrom pathlib import Path\np=Path("+repr(str(home))+ ")\nsignal.signal(signal.SIGINT,lambda *_:(p/'interrupt-received').write_text('yes'))\nprint('Nodalix terminal QA ready',flush=True)\nfor line in sys.stdin:\n (p/'terminal-input').write_text(line)\n")
                    terminal=subprocess.Popen(['ghostty','--confirm-close-surface=false','--keybind=shift+insert=paste_from_clipboard','--title=Nodalix Terminal QA',
                        '-e',sys.executable,'-u',str(terminal_program)],env=env,stdout=log,stderr=log)
                    try:
                        for attempt in range(100):
                            if evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()==='Nodalix Terminal QA')"):break
                            time.sleep(.05)
                        else:raise AssertionError('Real Ghostty did not map in the private Wayland session')
                        gtk=evaluate("(() => {const w=global.get_window_actors().find(a=>a.meta_window.get_title()==='Nodalix Terminal QA').meta_window;return [w.get_gtk_unique_bus_name(),w.get_gtk_window_object_path(),w.get_gtk_application_object_path()];})()")
                        actions={}
                        for label,path in zip(('window','application'),gtk[1:]):
                            result=bus.call_sync(gtk[0],path,'org.gtk.Actions','DescribeAll',None,None,Gio.DBusCallFlags.NONE,2000,None).unpack()[0]
                            actions[label]={name:value[0] for name,value in result.items()}
                        (args.evidence/'ghostty-actions.json').write_text(json.dumps(actions,indent=2))
                        time.sleep(.3);keys(ord('c'));time.sleep(.4)
                        assert not (home/'interrupt-received').exists(),'Super+C sent SIGINT to Ghostty'
                        action('prepare-copy');time.sleep(.3);keys(ord('c'));time.sleep(.3)
                        evaluate("global.get_window_actors().find(a=>a.meta_window.get_title()==='Nodalix Terminal QA').meta_window.activate(global.get_current_time()); true")
                        time.sleep(.3);keys(ord('v'));time.sleep(1);keys(0xff0d,());time.sleep(.4)
                        received=(home/'terminal-input').read_text()
                        assert received.strip()=='Nodalix copy QA',repr(received)
                        keys(ord('c'),(0xffe3,));time.sleep(.4)
                        assert (home/'interrupt-received').exists(),'Ctrl+C stopped sending SIGINT'
                        # A tab inside the focused window must not turn Super+Q into close-tab.
                        bus.call_sync(gtk[0],gtk[1],'org.gtk.Actions','Activate',
                            GLib.Variant('(sava{sv})',('new-tab',[],{})),None,Gio.DBusCallFlags.NONE,2000,None)
                        time.sleep(.4)
                        keys(ord('q'))
                        for attempt in range(40):
                            if not evaluate("global.get_window_actors().some(a=>a.meta_window.get_gtk_unique_bus_name()==="+json.dumps(gtk[0])+")"):break
                            time.sleep(.05)
                        else:raise AssertionError(evaluate("({focus:global.display.focus_window?.get_title(),mode:Main.actionMode,action:global.display.get_keybinding_action(24,64),windows:global.get_window_actors().map(a=>({title:a.meta_window.get_title(),bus:a.meta_window.get_gtk_unique_bus_name()}))})"))
                    finally:
                        terminal.terminate();terminal.wait(timeout=5)
                    action('prepare-copy');time.sleep(.3)
                    evaluate("global._qaAreaRequested=false; global._qaKeyboardEvents=[]; global._qaKeyTrace=global.stage.connect('captured-event',(_s,e)=>{if(e.type()===imports.gi.Clutter.EventType.KEY_PRESS)global._qaKeyboardEvents.push([e.get_key_symbol(),e.get_key_code(),e.get_state()]);return imports.gi.Clutter.EVENT_PROPAGATE;}); global._qaScreenshotOpen=Main.screenshotUI.open; Main.screenshotUI.open=function(...args){global._qaAreaRequested=true; return global._qaScreenshotOpen.apply(this,args);}; true")
                    keys(ord('4'),(0xffeb,0xffe1))
                    for attempt in range(40):
                        if evaluate('Main.screenshotUI.visible'):break
                        time.sleep(.1)
                    else:raise AssertionError(evaluate("({requested:global._qaAreaRequested,events:global._qaKeyboardEvents,action:global.display.get_keybinding_action(13,65),mode:Main.actionMode,visible:Main.screenshotUI.visible,area:Main.screenshotUI._selectionButton.checked,opening:Main.screenshotUI._openingCoroutineInProgress,keys:new imports.gi.Gio.Settings({schema_id:'org.gnome.shell.keybindings'}).get_strv('show-screenshot-ui')})"))
                    evaluate('Main.screenshotUI.open=global._qaScreenshotOpen; true')
                    evaluate('global.stage.disconnect(global._qaKeyTrace); true')
                    evaluate('Main.screenshotUI.close(true); true');time.sleep(.3)
                    keys(ord('3'),(0xffeb,0xffe1));time.sleep(.8)
                    # Capture storage is in the private home, never the user's Pictures.
                    captures=list(home.rglob('Screenshot*.png'))+list(home.rglob('Captura*.png'))
                    assert captures,'Native full-screen shortcut did not save a capture'
                    (args.evidence/'shortcuts.json').write_text(json.dumps({'real_gtk_super_copy_paste':True,
                        'real_ghostty_super_paste':True,'super_copy_preserves_terminal_process':True,'ctrl_c_still_sends_sigint':True,
                        'ghostty_super_q_closes_window_with_two_tabs':True,
                        'focus_mode':focus,'area_capture_ui':True,'full_screen_capture_saved':True},indent=2))
                finally:
                    entry.terminate();entry.wait(timeout=5)
                if importlib.util.find_spec('PySide6'):
                    qt_fixture="""from PySide6.QtWidgets import QApplication,QWidget,QLineEdit,QVBoxLayout
app=QApplication([])
win=QWidget();win.setWindowTitle('Nodalix Qt QA');win.resize(400,200)
entry=QLineEdit('Nodalix Qt clipboard QA');layout=QVBoxLayout(win);layout.addWidget(entry)
entry.textChanged.connect(lambda text:win.setWindowTitle('Nodalix Qt QA: '+text))
win.show();entry.setFocus();entry.selectAll();app.exec()
"""
                    qt=subprocess.Popen([sys.executable,'-c',qt_fixture],env={**env,'QT_QPA_PLATFORM':'wayland','QT_QPA_PLATFORMTHEME':''},stdout=log,stderr=log)
                    try:
                        for attempt in range(60):
                            if evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()?.startsWith('Nodalix Qt QA'))"):break
                            time.sleep(.05)
                        else:raise AssertionError('Qt clipboard fixture did not map')
                        evaluate("Main.overview.hide();global.get_window_actors().find(a=>a.meta_window.get_title()?.startsWith('Nodalix Qt QA')).meta_window.activate(global.get_current_time()); true")
                        time.sleep(.3);keys(ord('c'));time.sleep(.3)
                        keys(ord('a'),(0xffe3,));time.sleep(.1);keys(0xff08,());time.sleep(.2)
                        keys(ord('v'));time.sleep(.4)
                        assert evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()==='Nodalix Qt QA: Nodalix Qt clipboard QA')")
                        keys(ord('q'))
                        for attempt in range(40):
                            if not evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()?.startsWith('Nodalix Qt QA'))"):break
                            time.sleep(.05)
                        else:raise AssertionError(evaluate("({focus:global.display.focus_window?.get_title(),mode:Main.actionMode,action:global.display.get_keybinding_action(24,64),close:new imports.gi.Gio.Settings({schema_id:'org.gnome.desktop.wm.keybindings'}).get_strv('close')})"))
                        (args.evidence/'qt-shortcuts.json').write_text(json.dumps({'super_copy_paste':True,'super_q_closes_window':True}))
                    finally:
                        qt.terminate();qt.wait(timeout=5)
                native=os.environ.get('NODALIX_QA_NATIVE_BINARY')
                if native:
                    fake=subprocess.Popen([sys.executable,str(ROOT/'tests/native-restart-service.py')],env=env,stdout=log,stderr=log)
                    try:
                        for attempt in range(40):
                            owned=bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus',
                                'NameHasOwner',GLib.Variant('(s)',('com.nodalix.Settings',)),None,
                                Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                            if owned:break
                            time.sleep(.05)
                        else:raise AssertionError('Fake Settings service did not acquire its private bus name')
                        subprocess.run([native],env={**env,'GTK_A11Y':'none','GSK_RENDERER':'cairo'},
                                       stdout=log,stderr=log,timeout=15,check=True)
                        (args.evidence/'native-settings.json').write_text(json.dumps({'restart_action':True,'mandatory_restart_disables_updates':True,'automatic_cancel_restores_switch':True,'automatic_enable':True,'automatic_survives_page_recreation':True,'automatic_disable':True,'backend':'simulated; no host update or reboot','display':'private-wayland'},indent=2))
                    finally:
                        fake.terminate();fake.wait(timeout=5)
                tiling_fixture="""import gi
gi.require_version('Gtk','4.0')
from gi.repository import Gtk,Gio
app=Gtk.Application(application_id='com.nodalix.TilingQA')
count=0
def add(*args):
    global count
    count+=1
    win=Gtk.ApplicationWindow(application=app,title='Nodalix Tile QA '+str(count),default_width=200,default_height=120)
    win.present()
def activate(app):
    action=Gio.SimpleAction.new('add-window',None)
    action.connect('activate',add);app.add_action(action);add()
app.connect('activate',activate);app.run()
"""
                tiler="Main.extensionManager.lookup('nodalix-auto-tiling@getnodalia.com').stateObj"
                tile_windows="global.get_window_actors().map(a=>a.meta_window).filter(w=>w.get_title()?.startsWith('Nodalix Tile QA '))"
                evaluate(f"{tiler}._settings.set_boolean('enabled',true); true")
                tiles=subprocess.Popen([sys.executable,'-c',tiling_fixture],env=env,stdout=log,stderr=log)
                try:
                    counts=[]
                    for count in range(1,5):
                        if count>1:
                            bus.call_sync('com.nodalix.TilingQA','/com/nodalix/TilingQA','org.gtk.Actions',
                                'Activate',GLib.Variant('(sava{sv})',('add-window',[],{})),None,
                                Gio.DBusCallFlags.NONE,2000,None)
                        for attempt in range(40):
                            if evaluate(f'{tile_windows}.length')==count:break
                            time.sleep(.05)
                        else:raise AssertionError('Tiling fixture did not map '+str(count)+' windows')
                        for attempt in range(40):
                            result=evaluate(f"(() => {{const t={tiler};return {tile_windows}.map(w=>{{const g=t._windowGroups.get(w.get_stable_sequence()),r=w.get_frame_rect(),expected=g?.rects.get(w.get_stable_sequence());return {{actual:[r.x,r.y,r.width,r.height],expected:expected?[expected.x,expected.y,expected.width,expected.height]:null}};}});}})()")
                            if all(row['actual']==row['expected'] for row in result):break
                            time.sleep(.05)
                        assert all(row['actual']==row['expected'] for row in result),result
                        counts.append({'windows':count,'rectangles':result})
                    evaluate(f"""(async () => {{
                        const ext=Main.extensionManager.lookup('tiling-assistant@leleat-on-github');
                        const {{Rect}}=await import(ext.dir.get_uri()+'/src/extension/utility.js');
                        const w={tile_windows}[0],a=w.get_work_area_for_monitor(w.get_monitor());
                        global._qaTATarget=w;
                        const rect=new Rect(a.x,a.y,Math.floor(a.width/2),a.height);
                        await ext.stateObj._twm.tile(w,rect,{{openTilingPopup:false,skipAnim:true}});
                        await ext.stateObj._twm._waitForWindowToSettle(w,rect.addGaps(new Rect(a),w.get_monitor()),1000);
                    }})().then(()=>global._qaTAComplete=true).catch(e=>global._qaTAError=String(e));true""")
                    for _ in range(80):
                        if evaluate('!!global._qaTAComplete || !!global._qaTAError'):break
                        time.sleep(.05)
                    assert evaluate('!!global._qaTAComplete'),evaluate('global._qaTAError??null')
                    manual_rect=evaluate('(() => {let r=global._qaTATarget.get_frame_rect();return [r.x,r.y,r.width,r.height];})()')
                    evaluate(f'{tiler}._retileAll();true');time.sleep(.2)
                    assert evaluate('!!global._qaTATarget.isTiled')
                    after_manual=evaluate('(() => {let r=global._qaTATarget.get_frame_rect();return [r.x,r.y,r.width,r.height];})()')
                    assert after_manual==manual_rect,(manual_rect,after_manual)
                    evaluate("Main.extensionManager.lookup('tiling-assistant@leleat-on-github').stateObj._twm.untile(global._qaTATarget,{skipAnim:true});true")
                    evaluate(f'{tiler}._scheduleRetile();true');time.sleep(.3)
                    (args.evidence/'tiling-assistant.json').write_text(json.dumps({'enabled':True,'real_manual_placement':True,'manual_rect':manual_rect,'automatic_layout_preserves_manual_placement':True,'untile':True},indent=2))
                    evaluate('global._qaPointer=global.stage.get_context().get_backend().get_default_seat().create_virtual_device(imports.gi.Clutter.InputDeviceType.POINTER_DEVICE); true')
                    time.sleep(.3)
                    evaluate('Main.overview.hide(); true');time.sleep(.5)
                    for index in (0,1):
                        evaluate(f"(() => {{const r={tile_windows}[{index}].get_frame_rect();global._qaPointerTarget=[r.x+r.width/2,r.y+r.height/2];global._qaPointer.notify_absolute_motion(imports.gi.GLib.get_monotonic_time(),...global._qaPointerTarget);return true;}})()")
                        time.sleep(.2)
                        evaluate('global._qaPointer.notify_absolute_motion(imports.gi.GLib.get_monotonic_time(),global._qaPointerTarget[0],global._qaPointerTarget[1]); true')
                        time.sleep(.4)
                        assert evaluate(f'global.display.focus_window==={tile_windows}[{index}]'),evaluate(f"({{pointer:global.get_pointer(),requested:global._qaPointerTarget,nativeFocus:imports.gi.Meta.prefs_get_focus_mode(),actual:global.display.focus_window?.get_title(),target:{tile_windows}[{index}].get_title(),mode:Main.actionMode}})")
                    evaluate(f'{tile_windows}[0].minimize(); true');time.sleep(.4)
                    assert evaluate(f'{tile_windows}.filter(w=>!w.minimized).every(w=>{tiler}._windowGroups.has(w.get_stable_sequence()))')
                    evaluate(f'{tile_windows}[0].unminimize(); true');time.sleep(.4)
                    evaluate(f'{tile_windows}[0].move_to_monitor(Main.layoutManager.monitors.find(m=>m.index!=={tile_windows}[0].get_monitor()).index); true')
                    time.sleep(.4)
                    assert evaluate(f'new Set({tile_windows}.map(w=>{tiler}._windowGroups.get(w.get_stable_sequence()).monitor)).size')==2
                    evaluate(f"{tiler}._settings.set_boolean('enabled',false); true")
                    evaluate(f'{tile_windows}[0].activate(global.get_current_time()); true');time.sleep(.2)
                    keys(ord('q'));time.sleep(.5)
                    assert evaluate(f'{tile_windows}.length')==3,'Super+Q did not close exactly one focused window'
                    (args.evidence/'tiling.json').write_text(json.dumps({'layouts':counts,'minimize_restore':True,'two_monitor_groups':True,'disable_without_logout':True,'hover_focus_without_click':True,'super_q_closes_one_window':True},indent=2))
                finally:
                    tiles.terminate();tiles.wait(timeout=5)
                    evaluate(f"{tiler}._settings.set_boolean('enabled',false); true")
                menu_fixture="""import gi
gi.require_version('Gtk','3.0')
from gi.repository import Gtk,Gio
app=Gtk.Application(application_id='com.nodalix.MenuQA')
def activate(app):
    win=Gtk.ApplicationWindow(application=app,title='Nodalix Menu QA',default_width=300,default_height=200)
    action=Gio.SimpleAction.new('qa-command',None)
    action.connect('activate',lambda *args:win.set_title('Nodalix Menu QA activated'))
    app.add_action(action)
    disabled=Gio.SimpleAction.new('qa-disabled',None);disabled.set_enabled(False);app.add_action(disabled)
    items=Gio.Menu();items.append('QA command','app.qa-command');items.append('QA disabled','app.qa-disabled')
    menu=Gio.Menu();menu.append_submenu('QA',items);app.set_menubar(menu)
    win.show_all();win.present()
app.connect('activate',activate);app.run()
"""
                menus=subprocess.Popen([sys.executable,'-c',menu_fixture],env=env,stdout=log,stderr=log)
                menubar="Main.extensionManager.lookup('global-menu@unmade.space').stateObj._menubar"
                try:
                    for attempt in range(100):
                        if evaluate(f'{menubar}._native.hasMenus()'):break
                        time.sleep(.1)
                    else:raise AssertionError('Real GTK menu was not exported to the global bar')
                    tree=evaluate(f'{menubar}._cache._entries.get({menubar}._targetKey)')
                    assert any(item.get('label')=='QA' for item in tree),tree
                    evaluate(f"{menubar}._onTreeReceived([{{label:'Stale reply'}}],'not-the-focused-window'); true")
                    assert evaluate(f'{menubar}._native.hasMenus()')
                    action=evaluate(f"(() => {{const b={menubar},tree=b._cache._entries.get(b._targetKey);return tree.flatMap(n=>n.children??[]).find(n=>n.label==='QA command')?.key;}})()")
                    assert action,tree
                    assert any(item.get('label')=='QA disabled' and item.get('enabled') is False for top in tree for item in top.get('children',[])),tree
                    evaluate(f'{menubar}._proxy.ActivateMenuItem({json.dumps(action)}); true')
                    for attempt in range(40):
                        if evaluate("global.get_window_actors().some(a=>a.meta_window.get_title()==='Nodalix Menu QA activated')"):break
                        time.sleep(.05)
                    else:raise AssertionError('Global menu did not invoke the real GTK action')
                    (args.evidence/'global-menu.json').write_text(json.dumps({'gtk3_native_export':True,'real_action_invoked':True,'disabled_state_preserved':True,'stale_window_reply_ignored':True},indent=2))
                finally:
                    menus.terminate();menus.wait(timeout=5)
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
                evaluate('Main.overview.hide();true');time.sleep(.3)
                evaluate('(() => {let d=global._qaKeyboard,t=imports.gi.GLib.get_monotonic_time();d.notify_key(t,125,1);d.notify_key(t+30000,125,0);return true;})()')
                time.sleep(.6)
                assert evaluate('Main.overview.visible'),'Standalone Super did not open GNOME overview'
                evaluate('Main.overview.hide();true')
                (args.evidence/'lifecycle.json').write_text(json.dumps({'quick_settings_before':before,'quick_settings_after':after,'secondary_bars':bars,'enable_disable_cycles':3,'workareas':workareas,'monitor_reconfiguration':True,'fullscreen_hide_restore':True,'scale_2_supported_and_tested':scale_test},indent=2))
                print('Real GNOME 51: all extensions enabled; repeated lifecycle preserved Quick Settings; two-monitor bars passed')
            finally:
                if 'media_keys' in locals() and media_keys.poll() is None:
                    media_keys.terminate();media_keys.wait(timeout=5)
                shell.set_strv('enabled-extensions',[]);Gio.Settings.sync()
                time.sleep(.3)
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
    return 0


if __name__=='__main__':raise SystemExit(main())
