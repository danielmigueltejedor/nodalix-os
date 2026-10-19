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
    try:
        if p.stat().st_uid != UID:
            return None
        env = dict(item.split(b'=', 1) for item in
                   (p / 'environ').read_bytes().split(b'\0') if b'=' in item)
        appid = env.get(b'SteamAppId', b'')
        if not appid.isdigit() or appid == b'0':
            return None
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


def stop(_signum, _frame):
    global running
    running = False


def main():
    global lib
    lib = load_gamemode()
    tracked = {}
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    print('Automatic GameMode for Steam games started', flush=True)
    try:
        while running:
            for pid, stamp in list(tracked.items()):
                try:
                    if identity(PROC / str(pid)) == stamp:
                        # Re-register if the GameMode daemon was restarted.
                        if lib.real_gamemode_query_status_for(pid) != 2:
                            lib.real_gamemode_request_start_for(pid)
                        continue
                except (OSError, IndexError):
                    pass
                # GameMode reaps exited clients itself. Do not unregister a
                # reused PID that might now belong to another client.
                del tracked[pid]
                print(f'Game exited: PID {pid}', flush=True)
            for p in PROC.iterdir():
                if not p.name.isdigit() or int(p.name) in tracked:
                    continue
                info = game_info(p)
                if not info:
                    continue
                try:
                    stamp = identity(p)
                except (OSError, IndexError):
                    continue
                pid = int(p.name)
                if lib.real_gamemode_query_status_for(pid) == 2:
                    # A game using gamemoderun already owns its registration.
                    continue
                if lib.real_gamemode_request_start_for(pid) == 0:
                    tracked[pid] = stamp
                    print(f'GameMode enabled: app {info[0]}, {info[1]}, PID {pid}', flush=True)
                else:
                    print(f'GameMode request failed: {lib.real_gamemode_error_string()!r}', flush=True)
            time.sleep(5)
    finally:
        for pid, stamp in tracked.items():
            try:
                if identity(PROC / str(pid)) == stamp:
                    lib.real_gamemode_request_end_for(pid)
            except (OSError, IndexError):
                pass


if __name__ == '__main__':
    main()
