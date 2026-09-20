#!/usr/bin/env python3
"""Open the window overview; load compatible Hymission only at session startup."""
import json
from pathlib import Path
import subprocess
import sys

PLUGIN = Path('/usr/lib/nodalix/hymission/libhymission.so')
STAMP = Path('/usr/share/nodalix/hymission/hyprland-commit')

def query(*args):
    return json.loads(subprocess.check_output(['hyprctl', *args, '-j'], text=True))

def main():
    try:
        loaded = any('hymission' in str(item.get('name', '')).lower() for item in query('plugin', 'list'))
        if '--startup' in sys.argv:
            if loaded or not PLUGIN.exists() or not STAMP.exists():
                return 0
            version = query('version')
            if version.get('commit') != STAMP.read_text().strip():
                print('Hymission deferred: Hyprland build does not match the plugin.', file=sys.stderr)
                return 0
            return subprocess.call(['hyprctl', 'plugin', 'load', str(PLUGIN)])
        if loaded:
            result = subprocess.run(['hyprctl', 'dispatch', 'hl.plugin.hymission.toggle()'], capture_output=True)
            if result.returncode == 0:
                return 0
    except (OSError, ValueError, subprocess.SubprocessError):
        pass
    if '--startup' in sys.argv:
        return 0
    return subprocess.call(['qs', '-c', 'nodalix', 'ipc', 'call', 'overview', 'toggle'])

if __name__ == '__main__':
    raise SystemExit(main())
