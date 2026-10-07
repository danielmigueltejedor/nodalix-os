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

def user_script(source):
    return run('runuser','-u','migration-test','--','env','DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus','XDG_RUNTIME_DIR=/run/user/1000','/usr/bin/python3','-c',source)

try:
    deadline=time.monotonic()+360
    while time.monotonic()<deadline:
        if Path('/home/migration-test/.local/state/nodalix/migrations/gnome-0.3.0.done').exists() and Path('/run/user/1000/bus').exists():break
        time.sleep(2)
    else:raise AssertionError('GNOME first-login migration did not finish')
    assert 'active'==run('systemctl','is-active','gdm').strip()
    assert 'active'==run('systemctl','is-active','cups.socket').strip()
    assert not Path('/run/nodalix-updater/reboot-required').exists()
    installed=set(run('pacman','-Qq').splitlines())
    assert not any(name.startswith(('hypr','quickshell','greetd')) for name in installed)
    assert not Path('/home/migration-test/.config/hypr').exists()
    assert 'VERSION_ID="0.3.0"' in Path('/etc/nodalix-release').read_text()
    assert not json.loads(run('nodalix-updater','status','--json'))['reboot_mandatory']
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
        if any(info.get('state') in (3,4) for info in states.values()):raise AssertionError(states)
    except GLib.Error:
        pass
    time.sleep(1)
else:raise AssertionError(states)
assert Gio.Settings.new('org.gnome.shell').get_strv('favorite-apps')==['org.gnome.Nautilus.desktop']
assert Gio.Settings.new('org.gnome.desktop.interface').get_string('clock-format')=='24h'
assert Gio.Settings.new('io.github.jeffshee.hanabi-extension').get_string('video-path').endswith('nodalix-aurora-forest-4k.mp4')
print('All seven GNOME extensions active; wallpaper, clock and pinned apps migrated')
local=bus.call_sync('com.nodalix.LocalSend','/com/nodalix/LocalSend','com.nodalix.LocalSend1','GetStatus',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
assert json.loads(local)['enabled']
print('LocalSend quick settings bridge available')
video=next(Path('/usr/share/backgrounds/nodalix/Animados').glob('*.mp4'))
bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','SetWallpaper',GLib.Variant('(s)',(str(video),)),None,Gio.DBusCallFlags.NONE,5000,None)
for attempt in range(40):
    state=json.loads(bus.call_sync('com.nodalix.Settings','/com/nodalix/Settings','com.nodalix.Settings1','GetWallpaperState',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0])
    if state.get('active'):break
    time.sleep(.5)
else:raise AssertionError(state)
print('Animated wallpaper renderer active')
'''))
    print('NODALIX_VM_MIGRATION_PASS',flush=True)
except Exception:
    traceback.print_exc()
    print(run('journalctl','-b','--no-pager','-n','160','--grep=nodalix|gnome-session|gnome-shell|gdm'))
    print('NODALIX_VM_MIGRATION_FAIL',flush=True)
finally:
    subprocess.run(['systemctl','poweroff'],check=False)
