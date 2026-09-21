#!/usr/bin/env python3
"""Apply images and videos through the shared Skwd renderer service."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def configure(pause_fullscreen=True):
    config = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config')) / 'nodalix/wallpaper/config.json'
    config.parent.mkdir(parents=True, exist_ok=True)
    data = {
        'theme': {'policy': 'off', 'backend': 'off', 'targets': []},
        'paper': {'engine': 'skwd-paper', 'idlePauseSeconds': 30,
                  'performanceMode': True, 'videoMultiProcess': False},
        'playback': {'fullscreen': pause_fullscreen, 'fullscreenScope': 'all', 'resumeDelay': 1},
        'performance': {'batterySaver': True, 'batteryFps': 30,
                        'batteryVideoIdleSeconds': 15, 'batteryWallpaperPerformance': True},
        'wallpaperMute': True,
    }
    content = json.dumps(data, indent=2) + '\n'
    if not config.exists() or config.read_text() != content:
        temp = config.with_suffix('.tmp')
        temp.write_text(content)
        temp.replace(config)


def stop_legacy():
    # Retire only this user's obsolete wallpaper renderers after successful apply.
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            if entry.stat().st_uid != os.getuid():
                continue
            if (entry / 'comm').read_text().strip() not in ('hyprpaper', 'mpvpaper'):
                continue
            os.kill(int(entry.name), signal.SIGCONT)
            os.kill(int(entry.name), signal.SIGTERM)
        except (OSError, ProcessLookupError):
            continue


def apply(path, pause_fullscreen=True):
    media = Path(path).expanduser().resolve(strict=True)
    if not media.is_file():
        raise ValueError('Wallpaper must be a regular file')
    configure(pause_fullscreen)
    subprocess.run(['systemctl', '--user', 'start', 'nodalix-wallpaper.service'], check=True, timeout=15)
    # Readiness is not guaranteed by a Type=simple service start.
    for attempt in range(30):
        probe = subprocess.run(['skwd-helm', 'current', '--json'], capture_output=True, text=True, timeout=3)
        if probe.returncode == 0:
            break
        time.sleep(0.1)
    else:
        raise RuntimeError('Wallpaper engine did not become ready')
    subprocess.run(['skwd-helm', 'apply', str(media), '--mute'], check=True, timeout=30)
    # The apply RPC queues work; wait for the daemon's confirmed output state.
    for attempt in range(50):
        result = subprocess.run(['skwd-helm', 'current', '--json'], check=True, capture_output=True, text=True, timeout=3)
        outputs = json.loads(result.stdout).get('outputs', [])
        connected = [o for o in outputs if o.get('connected')]
        if connected and all(o.get('path') == str(media) for o in connected):
            stop_legacy()
            return
        time.sleep(0.1)
    raise RuntimeError('Wallpaper engine did not confirm all displays; previous renderer preserved')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', nargs='?')
    parser.add_argument('--pause-fullscreen', choices=('0', '1'), default='1')
    args = parser.parse_args()
    if args.path:
        apply(args.path, args.pause_fullscreen == '1')
    else:
        configure(args.pause_fullscreen == '1')

if __name__ == '__main__':
    main()
