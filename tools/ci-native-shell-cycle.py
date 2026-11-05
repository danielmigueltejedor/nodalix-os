#!/usr/bin/python3
"""Real lock/unlock and suspend/resume, exclusively inside a disposable VM."""
import json
import os
import pwd
import subprocess
import sys
import time
from pathlib import Path

QA_ID = 'nodalix-ci-native@example.invalid'
PHONE_ID = 'nodalix-connect@getnodalia.com'
UNLOCK_STATE = '''JSON.stringify({
    active: Main.screenShield.active, locked: Main.screenShield._isLocked,
    mode: Main.sessionMode.currentMode, hasDialog: !!Main.screenShield._dialog,
    pending: Main.screenShield._dialog?._authPrompt?._userVerifier?.hasPendingMessages ?? null,
    transitions: Main.screenShield._lockDialogGroup.get_transition('translation_y') !== null ||
                 Main.screenShield._lockDialogGroup.get_transition('opacity') !== null,
    groupVisible: Main.screenShield._lockDialogGroup.visible,
    limits: Main.timeLimitsManager?.shouldLockSession ?? null
})'''


def wait_for(predicate, description, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.5)
    raise RuntimeError('Timed out: ' + description)


def verify_user():
    from gi.repository import Gio, GLib

    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    settings = Gio.Settings.new('org.gnome.shell')
    screensaver = Gio.Settings.new('org.gnome.desktop.screensaver')
    original_enabled = settings.get_strv('enabled-extensions')
    original_lock = screensaver.get_boolean('lock-enabled')
    profile = json.loads(Path('/usr/share/nodalix/gnome-extensions.json').read_text())
    disabled = settings.get_strv('disabled-extensions')
    profile = [uuid for uuid in profile if uuid not in disabled and uuid != PHONE_ID]
    preferences = [('org.gnome.desktop.input-sources', 'sources'),
                   ('org.gnome.desktop.wm.preferences', 'focus-mode'),
                   ('org.gnome.settings-daemon.plugins.media-keys', 'custom-keybindings')]

    def snapshot():
        return [Gio.Settings.new(schema).get_value(key).unpack() for schema, key in preferences]

    def call(interface, method, parameters=None):
        return bus.call_sync('org.gnome.Shell', '/org/gnome/Shell', interface,
                             method, parameters, None, Gio.DBusCallFlags.NONE,
                             30000, None).unpack()

    def info(uuid):
        return call('org.gnome.Shell.Extensions', 'GetExtensionInfo',
                    GLib.Variant('(s)', (uuid,)))[0]

    def evaluate(code):
        success, result = call('org.gnome.Shell', 'Eval', GLib.Variant('(s)', (code,)))
        if not success:
            raise RuntimeError(('Guest Shell evaluation failed', result))
        return json.loads(result)

    def active_modules():
        return all(info(uuid).get('state') == 1 for uuid in profile)

    before = snapshot()
    try:
        settings.set_strv('enabled-extensions', [*original_enabled, QA_ID])
        Gio.Settings.sync()
        wait_for(lambda: info(QA_ID).get('state') == 1, 'guest test helper')
        phone_count = evaluate("Main.panel.statusArea.quickSettings.menu._grid.get_children().filter(a => a.title === 'Enlace móvil').length")
        if phone_count != (0 if PHONE_ID in disabled else 1):
            raise RuntimeError(('Native Phone Link tile count', phone_count))
        screensaver.set_boolean('lock-enabled', True)
        Gio.Settings.sync()
        bus.call_sync('org.gnome.ScreenSaver', '/org/gnome/ScreenSaver',
                      'org.gnome.ScreenSaver', 'Lock', None, None,
                      Gio.DBusCallFlags.NONE, 30000, None)
        wait_for(lambda: evaluate('Main.sessionMode.currentMode === "unlock-dialog"'), 'real lock screen')
        evaluate('Main.screenShield.deactivate(false); true')
        try:
            wait_for(lambda: evaluate('!Main.screenShield.active && Main.sessionMode.currentMode === "user"'),
                     'unlock transition', timeout=60)
        except RuntimeError:
            print('Unlock diagnostics:', evaluate(UNLOCK_STATE), flush=True)
            raise
        wait_for(active_modules, 'native modules after unlock')
        print('Native Shell lock/unlock passed', flush=True)
        # Suspend through logind so Shell and Hanabi receive PrepareForSleep.
        # The separate QMP observer wakes this VM after its ACPI S3 event.
        screensaver.set_boolean('lock-enabled', False)
        Gio.Settings.sync()
        (Path(os.environ['XDG_RUNTIME_DIR']) / 'nodalix-qa-suspend-ready').touch()
        # systemctl returns after queueing the operation; the root supervisor
        # supplies a marker only after the kernel reports successful resume.
        wait_for(lambda: Path('/run/nodalix-qa-resumed').is_file(), 'kernel resume', timeout=180)
        evaluate('Main.screenShield.deactivate(false); true')
        wait_for(active_modules, 'native modules after resume')
        if snapshot() != before:
            raise RuntimeError('Native module transitions changed user preferences')
        environment = subprocess.check_output(['systemctl', '--user', 'show-environment'], text=True)
        session = dict(line.split('=', 1) for line in environment.splitlines() if '=' in line)
        if session.get('XDG_SESSION_TYPE') != 'wayland':
            raise RuntimeError('Wayland session did not survive resume')
        if not (Path(os.environ['XDG_RUNTIME_DIR']) / session['WAYLAND_DISPLAY']).is_socket():
            raise RuntimeError('Wayland socket missing after resume')
        playing = bus.call_sync('io.github.jeffshee.HanabiRenderer', '/io/github/jeffshee/HanabiRenderer',
                                'org.freedesktop.DBus.Properties', 'Get',
                                GLib.Variant('(ss)', ('io.github.jeffshee.HanabiRenderer', 'isPlaying')),
                                None, Gio.DBusCallFlags.NONE, 30000, None).unpack()[0]
        if not isinstance(playing.unpack() if isinstance(playing, GLib.Variant) else playing, bool):
            raise RuntimeError('Animated wallpaper renderer unresponsive after resume')
        print('Native Shell suspend/resume, Wayland, wallpaper and preferences passed', flush=True)
    finally:
        screensaver.set_boolean('lock-enabled', original_lock)
        settings.set_strv('enabled-extensions', original_enabled)
        Gio.Settings.sync()


def main():
    if os.environ.get('NODALIX_CI_GUEST') != '1':
        raise SystemExit('Disposable CI guest required')
    if os.getuid() != 0:
        verify_user()
        return
    username = os.environ.get('NODALIX_CI_USER', 'migration-test')
    user = pwd.getpwnam(username)
    success = Path('/sys/power/suspend_stats/success')
    before = int(success.read_text())
    ready = Path(f'/run/user/{user.pw_uid}/nodalix-qa-suspend-ready')
    ready.unlink(missing_ok=True)
    Path('/run/nodalix-qa-resumed').unlink(missing_ok=True)
    with subprocess.Popen(['runuser', '-u', username, '--', 'env', 'NODALIX_CI_GUEST=1',
                           f'DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{user.pw_uid}/bus',
                           f'XDG_RUNTIME_DIR=/run/user/{user.pw_uid}', 'XDG_CURRENT_DESKTOP=GNOME',
                           '/usr/bin/python3', str(Path(__file__).resolve())]) as process:
        try:
            wait_for(lambda: ready.is_file() or process.poll() is not None,
                     'lock/unlock and suspend preparation', timeout=240)
            if process.poll() is not None:
                raise RuntimeError('Guest lock/unlock preparation failed')
            subprocess.run(['systemctl', 'suspend'], check=True, timeout=120)
            def resumed():
                if process.poll() is not None:
                    raise RuntimeError('Guest cycle stopped before successful suspend/resume')
                return int(success.read_text()) > before
            wait_for(resumed, 'successful kernel suspend/resume', timeout=240)
            Path('/run/nodalix-qa-resumed').touch()
            if process.wait(timeout=180):
                raise RuntimeError('Guest desktop did not recover after resume')
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=30)


if __name__ == '__main__':
    main()
