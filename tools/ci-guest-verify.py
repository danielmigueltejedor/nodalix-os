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
        if (GUEST_HOME/'.local/state/nodalix/migrations/gnome-0.3.1.done').exists() and Path('/run/user/'+GUEST_UID+'/bus').exists():break
        time.sleep(2)
    else:raise AssertionError('GNOME first-login migration did not finish')
    # Only the disposable QEMU guest: keep the *user* session active while
    # software-emulated GNOME, Hanabi and video decoding run for many minutes.
    # Otherwise GNOME locks the screen and disables extensions restricted to
    # sessionModes=['user'], yielding false negatives with state=2.
    if not LIVE:
        user_script("""from gi.repository import Gio
Gio.Settings.new('org.gnome.desktop.session').set_uint('idle-delay', 0)
Gio.Settings.new('org.gnome.desktop.screensaver').set_boolean('lock-enabled', False)
""")
    assert 'active'==run('systemctl','is-active','gdm').strip()
    assert 'active'==run('systemctl','is-active','cups.socket').strip()
    assert not Path('/run/nodalix-updater/reboot-required').exists()
    installed=set(run('pacman','-Qq').splitlines())
    assert not installed.intersection({'hyprland','hypridle','hyprlock','hyprpaper','hyprutils','quickshell','greetd','greetd-regreet','uwsm','aquamarine','mpvpaper','xdg-desktop-portal-hyprland'})
    assert not (GUEST_HOME/'.config/hypr').exists()
    if not LIVE:
        expected_version=os.environ['NODALIX_CI_VERSION']
        assert f'VERSION_ID="{expected_version}"' in Path('/etc/nodalix-release').read_text()
        assert not json.loads(run('nodalix-updater','status','--json'))['reboot_mandatory']
    fresh=os.environ.get('NODALIX_CI_FRESH')=='1'
    print(user_script('''
import json,time
from pathlib import Path
from gi.repository import Gio,GLib
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
profile=json.loads(Path('/usr/share/nodalix/gnome-extensions.json').read_text())
import subprocess,os
assert Gio.Settings.new('io.github.jeffshee.hanabi-extension').get_string('video-path').endswith('nodalix-aurora-forest-4k.mp4')
# TCG and llvmpipe emulate the GPU on two CPU cores. Exercise the actual
# renderer, selection and portals using a reduced sample of the verified 4K
# collection; this guest-only file never becomes part of release packages.
source=Path('/usr/share/backgrounds/nodalix/animated/nodalix-aurora-forest-4k.mp4')
video=Path.home()/'.local/share/backgrounds/nodalix/animated/nodalix-qa-sample.mp4'
video.parent.mkdir(parents=True,exist_ok=True)
subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(source),'-t','3','-vf','scale=640:-2,fps=12','-c:v','libx264','-preset','ultrafast','-an',str(video)],check=True,timeout=120)
# Prepare the real cached poster before making the bounded UI service call.
import sys
sys.path.insert(0,'/usr/lib/nodalix')
from wallpaper_poster import poster_for
assert poster_for(video).stat().st_size
video_preferences=Gio.Settings.new('io.github.jeffshee.hanabi-extension')
video_preferences.set_string('video-path',str(video))
Gio.Settings.sync()
print('Original wallpaper preference verified; reduced guest-only playback prepared')
deadline=time.monotonic()+240
states={}
while time.monotonic()<deadline:
    try:
        for uuid in profile:
            states[uuid]=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(uuid,)),None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
        if set(states)==set(profile) and all(info.get('state')==1 for info in states.values()):break
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
# The real renderer initializes Mesa/GStreamer under TCG. Give its actual
# D-Bus interfaces the VM's latency budget before changing an active video.
# The desktop API keeps its short interactive timeouts unchanged.
def wait_renderer():
    deadline=time.monotonic()+180
    last=None
    while time.monotonic()<deadline:
        try:
            info=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',('hanabi-extension@jeffshee.github.io',)),None,Gio.DBusCallFlags.NONE,10000,None).unpack()[0]
            if info.get('state') in (3,4):raise AssertionError(info)
            playing=bus.call_sync('io.github.jeffshee.HanabiRenderer','/io/github/jeffshee/HanabiRenderer','org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',('io.github.jeffshee.HanabiRenderer','isPlaying')),None,Gio.DBusCallFlags.NONE,10000,None).unpack()[0]
            if isinstance(playing,GLib.Variant):playing=playing.unpack()
            if info.get('state')==1 and isinstance(playing,bool):return playing
            last=info
        except GLib.Error as error:last=str(error)
        time.sleep(1)
    raise AssertionError(('Real renderer did not become responsive',last))
wait_renderer()
print('Real renderer completed software graphics initialization')
import shutil
selected=video.with_name('nodalix-qa-selected.mp4')
shutil.copyfile(video,selected)
assert poster_for(selected).stat().st_size
video=selected
bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','SetWallpaper',GLib.Variant('(s)',(str(video),)),None,Gio.DBusCallFlags.NONE,10000,None)
state=json.loads(bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','GetWallpaperState',None,None,Gio.DBusCallFlags.NONE,10000,None).unpack()[0])
assert state['available'] and state['requested'] and state['path']==str(video),state
wait_renderer()
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
