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

def require(condition, message):
    if not condition:
        raise RuntimeError(message)

GUEST_USER=os.environ.get('NODALIX_CI_USER','migration-test')
GUEST_HOME=Path('/home')/GUEST_USER
GUEST_UID=run('id','-u',GUEST_USER).strip()
LIVE=os.environ.get('NODALIX_CI_LIVE')=='1'

def user_script(source):
    return run('runuser','-u',GUEST_USER,'--','env','DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/'+GUEST_UID+'/bus','XDG_RUNTIME_DIR=/run/user/'+GUEST_UID,'NODALIX_CI_FRESH='+os.environ.get('NODALIX_CI_FRESH','0'),'XDG_CURRENT_DESKTOP=GNOME','/usr/bin/python3','-c',source)

try:
    deadline=time.monotonic()+360
    qa_idle_configured=False
    while time.monotonic()<deadline:
        if not qa_idle_configured and Path('/run/user/'+GUEST_UID+'/bus').exists():
            # Only this disposable guest: two-core TCG initialization can take
            # longer than the normal desktop idle timeout. Locking would switch
            # Shell mode and intentionally disable the user-only extensions.
            user_script("from gi.repository import Gio; Gio.Settings.new('org.gnome.desktop.session').set_uint('idle-delay',0); Gio.Settings.new('org.gnome.desktop.screensaver').set_boolean('lock-enabled',False); Gio.Settings.sync()")
            qa_idle_configured=True
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
    require(run('systemctl','is-active','gdm').strip() == 'active', 'GDM is inactive')
    cups=subprocess.run(['systemctl','is-active','cups.socket'],capture_output=True,text=True).stdout.strip()
    if cups != 'active':
        print(subprocess.run(['systemctl','status','cups.socket','--no-pager'],capture_output=True,text=True).stdout,flush=True)
    require(cups == 'active', f'CUPS socket is {cups}')
    require(not Path('/run/nodalix-updater/reboot-required').exists(), 'Stale restart marker')
    installed=set(run('pacman','-Qq').splitlines())
    require('nodalix-gnome-shell' in installed, 'The native Nodalix Shell package is missing')
    require('gnome-shell' not in installed, 'The stock Shell package is still installed')
    require(Path('/usr/share/nodalix/shell-build').is_file(), 'Native Shell build identity is missing')
    legacy = installed.intersection({'hyprland','hypridle','hyprlock','hyprpaper','hyprutils','quickshell','greetd','greetd-regreet','uwsm','aquamarine','mpvpaper','xdg-desktop-portal-hyprland'})
    require(not legacy, f'Legacy desktop packages remain: {sorted(legacy)}')
    require(not (GUEST_HOME/'.config/hypr').exists(), 'Legacy Hyprland configuration remains')
    if not LIVE:
        expected_version=os.environ['NODALIX_CI_VERSION']
        require(f'VERSION_ID="{expected_version}"' in Path('/etc/nodalix-release').read_text(),
                f'Installed Nodalix version does not match {expected_version}')
        require(not json.loads(run('nodalix-updater','status','--json'))['reboot_mandatory'],
                'Updater still requests a restart')
    fresh=os.environ.get('NODALIX_CI_FRESH')=='1'
    print(user_script('''
import json,time,subprocess,os
from pathlib import Path
from gi.repository import Gio,GLib
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
profile=json.loads(Path('/usr/share/nodalix/gnome-extensions.json').read_text())
phone_uuid='nodalix-connect@getnodalia.com'
native_profile=[uuid for uuid in profile if uuid!=phone_uuid]
def shell_call(method,parameters=None):
    # Software-rendered Shell on two emulated cores can stay busy past one
    # call timeout while services start; wait for it to answer, up to 3 minutes.
    deadline=time.monotonic()+180
    while True:
        try:
            return bus.call_sync('org.gnome.Shell','/org/gnome/Shell',
                'org.gnome.Shell.Extensions',method,parameters,None,
                Gio.DBusCallFlags.NONE,10000,None).unpack()[0]
        except GLib.Error as error:
            if not error.matches(Gio.io_error_quark(),Gio.IOErrorEnum.TIMED_OUT) or time.monotonic()>=deadline:
                raise
listed=shell_call('ListExtensions')
registry=json.loads(subprocess.check_output(['gresource','extract',
    '/usr/lib/gnome-shell/libshell-51.so','/org/gnome/shell/nodalix/modules.json'],text=True))
native_ids=set(registry['bundled']+registry['desktopPackages']+[phone_uuid])
assert not native_ids.intersection(listed),listed
print('Nodalix functions are built in and absent from the extension list')

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
        for uuid in native_profile:
            states[uuid]=bus.call_sync('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell.Extensions','GetExtensionInfo',GLib.Variant('(s)',(uuid,)),None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
        if set(states)==set(native_profile) and all(info.get('state')==1 for info in states.values()):break
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
interface=Gio.Settings.new('org.gnome.desktop.interface')
assert interface.get_default_value('cursor-theme').unpack()=='Nodalix'
assert interface.get_default_value('cursor-size').unpack()==30
cursor=Path('/usr/share/icons/Nodalix')
assert 'Capitaine Cursors' in (cursor/'index.theme').read_text()
for shape in ('default','pointer','grab','grabbing'):
    scalable=cursor/'cursors_scalable'/shape
    frames=json.loads((scalable/'metadata.json').read_text())
    assert frames and all((scalable/frame['filename']).is_file() for frame in frames)
    assert (cursor/'cursors'/shape).read_bytes()[:4]==b'Xcur'
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
assert blur.get_int('settings-version')>=8
assert not source.lookup('org.gnome.shell.extensions.blur-my-shell',False).has_key('rounded-blur-found')
panel=Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.blur-my-shell.panel',False),None,None)
assert panel.get_boolean('blur')
video_preferences=Gio.Settings.new('io.github.jeffshee.hanabi-extension')
assert video_preferences.get_boolean('enable-va') and not video_preferences.get_boolean('prefer-clappersink')
assert Path.home().joinpath('.config/xdg-terminals.list').read_text().strip()=='com.mitchellh.ghostty.desktop'
print('GNOME profile, official blur schemas, native panel blur, shortcuts and repeated user migration passed')
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
# The serial verifier can inherit XDG_SESSION_TYPE=tty from PAM. The graphical
# session has already populated the user manager; preserve its real environment.
manager_environment=subprocess.check_output(['systemctl','--user','show-environment'],
                                            text=True,timeout=20)
session=dict(line.split('=',1) for line in manager_environment.splitlines() if '=' in line)
display=session.get('WAYLAND_DISPLAY','')
assert session.get('XDG_SESSION_TYPE')=='wayland' and display, 'GNOME did not export its Wayland session'
assert (Path(os.environ['XDG_RUNTIME_DIR'])/display).is_socket(), 'GNOME Wayland socket is missing'
# Let D-Bus activate the portals in the running graphical session. A forced
# restart can block waiting on GTK and the compositor in a software-rendered VM.
def wait_portal_interface(name,interface):
    deadline=time.monotonic()+180
    last=None
    while time.monotonic()<deadline:
        try:
            version=bus.call_sync(name,'/org/freedesktop/portal/desktop',
                'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',(interface,'version')),
                None,Gio.DBusCallFlags.NONE,10000,None).unpack()[0]
            if isinstance(version,GLib.Variant):version=version.unpack()
            if isinstance(version,int) and version>=1:return
            last=version
        except GLib.Error as error:last=str(error)
        time.sleep(1)
    subprocess.run(['journalctl','--user','-b','--no-pager','-n','60',
                    '-u','xdg-desktop-portal-gnome.service','-u','xdg-desktop-portal.service'],
                   check=False,timeout=10)
    raise AssertionError(('Screenshot portal interface did not become ready',name,interface,last))
wait_portal_interface('org.freedesktop.impl.portal.desktop.gnome','org.freedesktop.impl.portal.Screenshot')
wait_portal_interface('org.freedesktop.portal.Desktop','org.freedesktop.portal.Screenshot')
print('Real screenshot backend and frontend interfaces ready')
# The application catalog must be populated by the packaged session service
# on both a clean account and an upgraded account.
state_home=Path.home()/'.local/state/nodalix'
deadline=time.monotonic()+45
while time.monotonic()<deadline:
    try:
        layout=json.loads((state_home/'layout/paths.json').read_text())
        folder=Path(layout['applications'])
        registry=json.loads((state_home/'applications/registry.json').read_text())
        if folder.is_dir() and (folder/registry['org.gnome.Settings.desktop']['filename']).is_file():break
    except (OSError,ValueError,KeyError):
        pass
    time.sleep(.5)
else:raise AssertionError('The Applications folder did not acquire its native Settings entry')
assert subprocess.check_output(['systemctl','--user','is-active','nodalix-applications.service'],text=True).strip()=='active'
# The first query can activate Settings on a busy software-rendered VM.
deadline=time.monotonic()+90
while True:
    try:
        results=bus.call_sync('org.gnome.Settings.SearchProvider','/org/gnome/Settings/SearchProvider',
            'org.gnome.Shell.SearchProvider2','GetInitialResultSet',GLib.Variant('(as)',(['documentos','grandes'],)),
            None,Gio.DBusCallFlags.NONE,30000,None).unpack()[0]
        break
    except GLib.Error as error:
        if time.monotonic()>=deadline or not (error.matches(Gio.io_error_quark(),Gio.IOErrorEnum.TIMED_OUT)
                or error.matches(Gio.dbus_error_quark(),Gio.DBusError.NO_REPLY)):
            raise
assert 'nodalix-storage' in results,results
print('Applications folder and native Settings search are available')
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
    if not LIVE:
        print(run('/usr/bin/python3','/usr/local/lib/nodalix/ci-native-shell-cycle.py'))
    missing = {'xdg-desktop-portal','xdg-desktop-portal-gnome','xdg-desktop-portal-gtk'} - installed
    require(not missing, f'Missing desktop portals: {sorted(missing)}')
    print(user_script("print(__import__('subprocess').check_output(['systemctl','--user','is-active','xdg-desktop-portal.service','xdg-desktop-portal-gnome.service'],text=True))"))
    print('NODALIX_VM_MIGRATION_PASS',flush=True)
except Exception:
    traceback.print_exc()
    print(run('journalctl','-b','--no-pager','-n','100','_COMM=gnome-shell'))
    print(run('journalctl','-b','--no-pager','-n','90','--grep=nodalix|gnome-session|gdm'))
    print(run('journalctl','-b','--no-pager','-n','100','--user-unit=xdg-desktop-portal.service','--user-unit=xdg-desktop-portal-gnome.service'))
    print('NODALIX_VM_MIGRATION_FAIL',flush=True)
finally:
    subprocess.run(['systemctl','poweroff'],check=False)
