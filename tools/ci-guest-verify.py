#!/usr/bin/python3
"""Checks inside the disposable QEMU guest after a real reboot/login."""
import json,os,subprocess,time,traceback
from pathlib import Path
from gi.repository import Gio,GLib

def run(*args,**kwargs):
    result=subprocess.run(args,capture_output=True,text=True,**kwargs)
    if result.returncode:
        print(result.stdout+result.stderr,flush=True)
        result.check_returncode()
    return result.stdout

GUEST_USER=os.environ.get('NODALIX_CI_USER','migration-test')
GUEST_HOME=Path('/home')/GUEST_USER
GUEST_UID=run('id','-u',GUEST_USER).strip()
LIVE=os.environ.get('NODALIX_CI_LIVE')=='1'

def user_script(source):
    return run('runuser','-u',GUEST_USER,'--','env','DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/'+GUEST_UID+'/bus','XDG_RUNTIME_DIR=/run/user/'+GUEST_UID,'NODALIX_CI_FRESH='+os.environ.get('NODALIX_CI_FRESH','0'),'XDG_CURRENT_DESKTOP=GNOME','/usr/bin/python3','-c',source)

try:
    deadline=time.monotonic()+360
    while time.monotonic()<deadline:
        if (GUEST_HOME/'.local/state/nodalix/migrations/gnome-0.3.0-v2.done').exists() and Path('/run/user/'+GUEST_UID+'/bus').exists():break
        time.sleep(2)
    else:raise AssertionError('GNOME first-login migration did not finish')
    assert 'active'==run('systemctl','is-active','gdm').strip()
    assert 'active'==run('systemctl','is-active','cups.socket').strip()
    assert not Path('/run/nodalix-updater/reboot-required').exists()
    installed=set(run('pacman','-Qq').splitlines())
    assert not installed.intersection({'hyprland','hypridle','hyprlock','hyprpaper','hyprutils','quickshell','greetd','greetd-regreet','uwsm','aquamarine','mpvpaper','xdg-desktop-portal-hyprland'})
    assert not (GUEST_HOME/'.config/hypr').exists()
    if not LIVE:
        assert 'VERSION_ID="0.3.0"' in Path('/etc/nodalix-release').read_text()
        assert not json.loads(run('nodalix-updater','status','--json'))['reboot_mandatory']
    fresh=os.environ.get('NODALIX_CI_FRESH')=='1'
    print(user_script('''
import json,time
from pathlib import Path
from gi.repository import Gio,GLib
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
profile=json.loads(Path('/usr/share/nodalix/gnome-extensions.json').read_text())
deadline=time.monotonic()+150
states={}
while time.monotonic()<deadline:
    try:
        states={uuid:bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(uuid,)),None,Gio.DBusCallFlags.NONE,1500,None).unpack()[0] for uuid in profile}
        if all(info.get('state')==1 for info in states.values()):break
        # GNOME uses ERROR with an empty error while async module imports run.
        if any(info.get('state')==4 or info.get('error') for info in states.values()):raise AssertionError(states)
    except GLib.Error:
        pass
    time.sleep(1)
else:raise AssertionError(states)
import os
if os.environ.get('NODALIX_CI_FRESH')!='1':
    assert Gio.Settings.new('org.gnome.shell').get_strv('favorite-apps')==['org.gnome.Nautilus.desktop']
assert Gio.Settings.new('org.gnome.desktop.interface').get_string('clock-format')=='24h'
assert Gio.Settings.new('io.github.jeffshee.hanabi-extension').get_string('video-path').endswith('nodalix-aurora-forest-4k.mp4')
assert Gio.Settings.new('org.gnome.desktop.input-sources').get_value('sources').unpack()
preferences=[('org.gnome.desktop.input-sources','sources'),('org.gnome.desktop.wm.preferences','focus-mode'),('org.gnome.settings-daemon.plugins.media-keys','custom-keybindings')]
before=[Gio.Settings.new(schema).get_value(key).unpack() for schema,key in preferences]
import subprocess
subprocess.run(['/usr/lib/nodalix/nodalix-gnome-user-migrate'],check=True)
subprocess.run(['/usr/lib/nodalix/nodalix-gnome-user-migrate'],check=True)
assert before==[Gio.Settings.new(schema).get_value(key).unpack() for schema,key in preferences]
assert Gio.Settings.new('org.gnome.desktop.wm.preferences').get_string('button-layout')==':minimize,maximize,close'
source=Gio.SettingsSchemaSource.new_from_directory('/usr/share/gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas',Gio.SettingsSchemaSource.get_default(),False)
dock=Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.dash-to-dock',False),None,None)
assert not dock.get_boolean('dock-fixed') and dock.get_boolean('autohide') and dock.get_boolean('intellihide')
source=Gio.SettingsSchemaSource.new_from_directory('/usr/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas',Gio.SettingsSchemaSource.get_default(),False)
blur=Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.blur-my-shell',False),None,None)
assert blur.get_boolean('rounded-blur-found')
video_preferences=Gio.Settings.new('io.github.jeffshee.hanabi-extension')
assert video_preferences.get_boolean('enable-va') and not video_preferences.get_boolean('prefer-clappersink')
assert Path.home().joinpath('.config/xdg-terminals.list').read_text().strip()=='com.mitchellh.ghostty.desktop'
print('GNOME profile, private schemas, rounded blur, shortcuts and repeated user migration passed')
deadline=time.monotonic()+90
while time.monotonic()<deadline:
    try:
        local=bus.call_sync('com.nodalix.LocalSend','/com/nodalix/LocalSend','com.nodalix.LocalSend1','GetStatus',None,None,Gio.DBusCallFlags.NONE,10000,None).unpack()[0]
        if json.loads(local)['enabled']:break
    except GLib.Error:pass  # Software-rendered VM startup can delay the shell bus.
    time.sleep(.5)
else:raise AssertionError('LocalSend did not start after its configured startup delay')
print('LocalSend quick settings bridge available')
video=next(Path('/usr/share/backgrounds/nodalix/animated').glob('*.mp4'))
bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','SetWallpaper',GLib.Variant('(s)',(str(video),)),None,Gio.DBusCallFlags.NONE,5000,None)
for attempt in range(40):
    state=json.loads(bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','GetWallpaperState',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0])
    if state.get('active'):break
    time.sleep(.5)
else:raise AssertionError(state)
print('Animated wallpaper renderer active')
# Register this QA process explicitly; the systemd verifier otherwise supplies
# a different app identity than a terminal process. All files are guest-only.
qa_id='com.nodalix.QA'
desktop=Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share')))/'applications'/f'{qa_id}.desktop'
desktop.parent.mkdir(parents=True,exist_ok=True)
desktop.write_text(chr(10).join(['[Desktop Entry]','Type=Application','Name=Nodalix QA','Exec=/usr/bin/true','']))
time.sleep(1)
bus.call_sync('org.freedesktop.portal.Desktop','/org/freedesktop/portal/desktop','org.freedesktop.host.portal.Registry','Register',GLib.Variant('(sa{sv})',(qa_id,{})),None,Gio.DBusCallFlags.NONE,60000,None)
# Permission fixture belongs only to this disposable VM user's database.
bus.call_sync('org.freedesktop.impl.portal.PermissionStore','/org/freedesktop/impl/portal/PermissionStore','org.freedesktop.impl.portal.PermissionStore','Set',GLib.Variant('(sbsa{sas}v)',('screenshot',True,'screenshot',{qa_id:['yes']},GLib.Variant('s',''))),None,Gio.DBusCallFlags.NONE,5000,None)
loop=GLib.MainLoop();responses=[]
def responded(connection,sender,path,interface,signal,parameters):
    responses.append(parameters.unpack());loop.quit()
token='nodalixqa'
request='/org/freedesktop/portal/desktop/request/'+bus.get_unique_name().lstrip(':').replace('.','_')+'/'+token
subscription=bus.signal_subscribe(None,'org.freedesktop.portal.Request','Response',request,None,Gio.DBusSignalFlags.NONE,responded)
result=bus.call_sync('org.freedesktop.portal.Desktop','/org/freedesktop/portal/desktop','org.freedesktop.portal.Screenshot','Screenshot',GLib.Variant('(sa{sv})',('',{'handle_token':GLib.Variant('s',token),'interactive':GLib.Variant('b',False)})),None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
assert result==request
GLib.timeout_add_seconds(90,lambda:(loop.quit(),False)[1]);loop.run()
bus.signal_unsubscribe(subscription)
assert responses and responses[0][0]==0,responses
from urllib.parse import urlparse,unquote
screenshot=Path(unquote(urlparse(responses[0][1]['uri']).path))
assert screenshot.read_bytes().startswith(bytes([137,80,78,71]))
print('Real GNOME screenshot portal produced a PNG')
'''))
    assert {'xdg-desktop-portal','xdg-desktop-portal-gnome','xdg-desktop-portal-gtk'}.issubset(installed)
    print(user_script("print(__import__('subprocess').check_output(['systemctl','--user','is-active','xdg-desktop-portal.service','xdg-desktop-portal-gnome.service'],text=True))"))
    print('NODALIX_VM_MIGRATION_PASS',flush=True)
except Exception:
    traceback.print_exc()
    print(run('journalctl','-b','--no-pager','-n','100','_COMM=gnome-shell'))
    print(run('journalctl','-b','--no-pager','-n','90','--grep=nodalix|gnome-session|gdm'))
    print('NODALIX_VM_MIGRATION_FAIL',flush=True)
finally:
    subprocess.run(['systemctl','poweroff'],check=False)
