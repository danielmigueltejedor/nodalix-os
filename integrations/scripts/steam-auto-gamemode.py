#!/usr/bin/python
"""Request GameMode for running Steam game executables, without editing Steam."""
import ctypes
import os
from pathlib import Path
import signal
import time

UID = os.getuid()
PROC = Path('/proc')
running = True
lib = None

# Hold a temporary GNOME performance profile on the system bus, rather than
# setting ActiveProfile permanently. The daemon automatically restores the
# user's selected profile when the hold is released or this process exits.
PROFILE_BUS = 'org.freedesktop.UPower.PowerProfiles'
PROFILE_PATH = '/org/freedesktop/UPower/PowerProfiles'
PROFILE_IFACE = 'org.freedesktop.UPower.PowerProfiles'


class PerformanceProfileHold:
    """One D-Bus profile reservation while any tracked Steam game is running.

    A manual user profile switch cancels the hold. Do not override that choice
    again until all games from the current gaming session have exited.
    """

    def __init__(self):
        self.bus = None
        self.gio = None
        self.glib = None
        self.cookie = None
        self.user_overrode = False
        self.retry_after = 0.0

    def _profile_released(self, _conn, _sender, _path, _iface, _signal, params, _data):
        (cookie,) = params.unpack()
        if cookie == self.cookie:
            self.cookie = None
            self.user_overrode = True
            print('User changed power profile; automatic performance paused for this game session', flush=True)

    def _owner_changed(self, _conn, _sender, _path, _iface, _signal, params, _data):
        name, old_owner, new_owner = params.unpack()
        if name == PROFILE_BUS and old_owner != new_owner:
            # A first activation can be signalled after HoldProfile returns.
            # Only the loss of an existing daemon invalidates its holds.
            if old_owner:
                self.cookie = None
            self.retry_after = 0.0

    def _connect(self):
        if self.bus is not None:
            return
        # python-gobject is already a dependency of nodalix-integrations.
        from gi.repository import Gio, GLib

        bus = Gio.bus_get_sync(Gio.BusType.SYSTEM, None)
        bus.signal_subscribe(
            PROFILE_BUS, PROFILE_IFACE, 'ProfileReleased', PROFILE_PATH,
            None, Gio.DBusSignalFlags.NONE, self._profile_released,
        )
        bus.signal_subscribe(
            'org.freedesktop.DBus', 'org.freedesktop.DBus', 'NameOwnerChanged',
            '/org/freedesktop/DBus', PROFILE_BUS,
            Gio.DBusSignalFlags.NONE, self._owner_changed,
        )
        self.bus = bus
        self.gio = Gio
        self.glib = GLib

    def _dispatch_signals(self):
        if self.glib is None:
            return
        context = self.glib.MainContext.default()
        # The detector runs without a GLib main loop; process D-Bus events
        # without blocking each polling cycle.
        for _ in range(32):
            if not context.pending():
                break
            context.iteration(False)

    def sync(self, games_active):
        self._dispatch_signals()
        if not games_active:
            self.release()
            self.user_overrode = False
            self.retry_after = 0.0
            return
        if self.cookie is not None or self.user_overrode:
            return
        if time.monotonic() < self.retry_after:
            return
        try:
            self._connect()
            result = self.bus.call_sync(
                PROFILE_BUS, PROFILE_PATH, PROFILE_IFACE, 'HoldProfile',
                self.glib.Variant('(sss)', (
                    'performance', 'Steam game running', 'com.nodalix.SteamAutoGameMode',
                )),
                self.glib.VariantType('(u)'), self.gio.DBusCallFlags.NONE,
                3000, None,
            )
            self.cookie = result.unpack()[0]
            print('GNOME power profile: performance hold enabled', flush=True)
        except Exception as error:
            # GameMode still works even if the power-profile service is absent.
            self.retry_after = time.monotonic() + 30
            print(f'GNOME performance hold unavailable: {error}', flush=True)

    def release(self):
        self._dispatch_signals()
        cookie = self.cookie
        self.cookie = None
        if cookie is None or self.bus is None:
            return
        try:
            self.bus.call_sync(
                PROFILE_BUS, PROFILE_PATH, PROFILE_IFACE, 'ReleaseProfile',
                self.glib.Variant('(u)', (cookie,)), None,
                self.gio.DBusCallFlags.NONE, 3000, None,
            )
            print('GNOME power profile: performance hold released', flush=True)
        except Exception as error:
            # The daemon may have restarted or the user may have switched
            # profiles before its ProfileReleased signal was dispatched.
            print(f'GNOME performance hold release skipped: {error}', flush=True)


def load_gamemode():
    lib = ctypes.CDLL('libgamemode.so.0')
    for name in ('request_start_for', 'request_end_for', 'query_status_for'):
        fn = getattr(lib, 'real_gamemode_' + name)
        fn.argtypes = [ctypes.c_int]
        fn.restype = ctypes.c_int
    lib.real_gamemode_error_string.restype = ctypes.c_char_p
    return lib


def identity(p):
    # A PID can be reused; pair it with its kernel start time.
    return (p / 'stat').read_text().rsplit(')', 1)[1].split()[19]


def game_info(p):
    return judge(p)[0]


def judge(p):
    """Return (game info or None, whether a None verdict is final).

    A process without a usable SteamAppId can never become a game, because the
    environment is fixed at exec(). One that does belong to a Steam app may
    still be starting (Wine maps the game executable late), so it is re-checked.
    """
    try:
        if p.stat().st_uid != UID:
            return None, True
        raw_env = (p / 'environ').read_bytes()
        if not (raw_env.startswith(b'SteamAppId=') or b'\0SteamAppId=' in raw_env):
            return None, True
        env = dict(item.split(b'=', 1) for item in raw_env.split(b'\0') if b'=' in item)
        appid = env.get(b'SteamAppId', b'')
        if not appid.isdigit() or appid == b'0':
            return None, True
        return _steam_game(p, env, appid), False
    except (OSError, ValueError, IndexError):
        return None, False


def _steam_game(p, env, appid):
    try:
        exe = os.readlink(p / 'exe')
        install = os.fsdecode(env.get(b'STEAM_COMPAT_INSTALL_PATH', b''))
        if not install:
            # Native Steam games normally execute from their install directory.
            marker = '/steamapps/common/'
            if marker not in exe:
                return None
            prefix, tail = exe.split(marker, 1)
            directory = tail.split('/', 1)[0]
            if directory.lower().startswith(('proton', 'steamlinuxruntime', 'steamworks')):
                return None
            install = prefix + marker + directory
        install = install.rstrip('/') + '/'
        if exe.startswith(install) and 'wine' not in Path(exe).name.lower():
            return appid.decode(), Path(exe).name
        # Proton's /proc/exe points at Wine. Confirm that the actual Windows
        # executable is mapped from this game's install directory.
        if Path(exe).name not in ('wine', 'wine64', 'wine-preloader', 'wine64-preloader'):
            return None
        for line in (p / 'maps').read_text().splitlines():
            fields = line.split(None, 5)
            if len(fields) == 6:
                mapped = fields[5].removesuffix(' (deleted)')
                if mapped.startswith(install) and mapped.lower().endswith('.exe'):
                    if any(s in Path(mapped).name.lower() for s in
                           ('crashreport', 'crashpad', 'unins', 'setup')):
                        continue
                    return appid.decode(), Path(mapped).name
    except (OSError, ValueError, IndexError):
        pass
    return None


ACTIVE_INTERVAL = 5   # seconds between scans while a game is tracked
IDLE_INTERVAL = 10    # a game only needs GameMode a few seconds after it starts


NEGATIVE_TTL = 300    # re-judge long-lived non-games even if exec() looks identical


def process_key(p):
    """Identity of what a PID currently runs: a new exec() changes it.

    The stack and argument/environment addresses are re-randomised by every
    exec(), so re-running the same executable with a different environment
    still produces a new key. Without ASLR the TTL below is the safety net.
    """
    fields = (p / 'stat').read_text().rsplit(')', 1)[1].split()
    return fields[19], fields[25], fields[45], fields[47], os.readlink(p / 'exe')


def candidate_processes(tracked, ignored):
    """Yield user processes not yet judged, without re-reading their environment.

    SteamAppId is fixed when a program is exec()ed, so a process that was
    judged not to be a game stays that way until it exec()s again (see
    process_key) or NEGATIVE_TTL expires. This avoids reading every process
    environment on every scan.
    """
    live = set()
    for p in PROC.iterdir():
        if not p.name.isdigit():
            continue
        pid = int(p.name)
        live.add(pid)
        if pid in tracked:
            continue
        if pid in ignored:
            try:
                key, expires = ignored[pid]
                if process_key(p) == key and time.monotonic() < expires:
                    continue
            except (OSError, IndexError):
                pass
            del ignored[pid]
        yield p
    for pid in set(ignored) - live:
        del ignored[pid]


def stop(_signum, _frame):
    global running
    running = False


def main():
    global lib
    lib = load_gamemode()
    tracked = {}  # PID -> (kernel start time, GameMode registration owned by us)
    ignored = {}  # PID -> (process key, expiry) already judged not to be a game
    power = PerformanceProfileHold()
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    print('Automatic GameMode for Steam games started', flush=True)
    try:
        while running:
            for pid, (stamp, owned) in list(tracked.items()):
                try:
                    if identity(PROC / str(pid)) == stamp:
                        # Re-register only requests this helper owns.
                        if owned and lib.real_gamemode_query_status_for(pid) != 2:
                            lib.real_gamemode_request_start_for(pid)
                        continue
                except (OSError, IndexError):
                    pass
                # GameMode reaps exited clients itself. Do not unregister a
                # reused PID that might now belong to another client.
                del tracked[pid]
                print(f'Game exited: PID {pid}', flush=True)
            for p in candidate_processes(tracked, ignored):
                info, final = judge(p)
                if not info:
                    if final:
                        try:
                            ignored[int(p.name)] = (process_key(p), time.monotonic() + NEGATIVE_TTL)
                        except (OSError, IndexError):
                            pass
                    continue
                try:
                    stamp = identity(p)
                except (OSError, IndexError):
                    continue
                pid = int(p.name)
                if lib.real_gamemode_query_status_for(pid) == 2:
                    # Also track games launched with gamemoderun, but do not
                    # modify a GameMode registration owned by another process.
                    tracked[pid] = (stamp, False)
                    print(f'GameMode already active: app {info[0]}, {info[1]}, PID {pid}', flush=True)
                    continue
                if lib.real_gamemode_request_start_for(pid) == 0:
                    tracked[pid] = (stamp, True)
                    print(f'GameMode enabled: app {info[0]}, {info[1]}, PID {pid}', flush=True)
                else:
                    print(f'GameMode request failed: {lib.real_gamemode_error_string()!r}', flush=True)
            power.sync(bool(tracked))
            time.sleep(ACTIVE_INTERVAL if tracked else IDLE_INTERVAL)
    finally:
        # Release the profile even when this service is stopped while a game
        # is running; the system bus also releases it if our process crashes.
        power.release()
        for pid, (stamp, owned) in tracked.items():
            try:
                if owned and identity(PROC / str(pid)) == stamp:
                    lib.real_gamemode_request_end_for(pid)
            except (OSError, IndexError):
                pass


if __name__ == '__main__':
    main()
