#!/usr/bin/env python3
"""Load native minimize compatibility only for the exact compositor build."""
import json
from pathlib import Path
import subprocess

PLUGIN = Path('/usr/lib/nodalix/window-controls/libnodalix-window-controls.so')
STAMP = Path('/usr/share/nodalix/window-controls/hyprland-commit')

def main():
    try:
        loaded = json.loads(subprocess.check_output(['hyprctl', 'plugin', 'list', '-j'], text=True))
        if any(p.get('name') == 'nodalix-window-controls' for p in loaded):
            return 0
        if not PLUGIN.exists() or not STAMP.exists():
            return 0
        version = json.loads(subprocess.check_output(['hyprctl', 'version', '-j'], text=True))
        if version.get('commit') != STAMP.read_text().strip():
            print('Native minimize deferred: update the Nodalix package for this Hyprland build.')
            return 0
        result = subprocess.run(['hyprctl', 'plugin', 'load', str(PLUGIN)], capture_output=True, text=True)
        if result.returncode or 'error' in result.stdout.lower():
            print('Native minimize compatibility could not load: ' + result.stdout.strip())
            return 1
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print('Native minimize compatibility unavailable: ' + str(error))
        return 1

if __name__ == '__main__':
    raise SystemExit(main())
