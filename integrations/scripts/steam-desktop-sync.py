#!/usr/bin/python3
"""Keep installed Steam games associated with their GNOME launchers, offline."""
import argparse
import configparser
import fcntl
import os
from pathlib import Path
import re
import subprocess
import tempfile


TOKEN = re.compile(r'//[^\n]*|"((?:\\.|[^"\\])*)"|([{}])')
GAME_URI = re.compile(r'steam://(?:rungameid|run)/(\d+)\b')
TOOLS = re.compile(r'^(?:Steam Linux Runtime|Steamworks Common Redistributables|Proton(?:\s|$))', re.I)


def vdf(text):
    tokens = [m.group(2) or re.sub(r'\\(["\\])', r'\1', m.group(1))
              for m in TOKEN.finditer(text) if not m.group(0).startswith('//')]
    pos = 0

    def obj(nested=False):
        nonlocal pos
        result = {}
        while pos < len(tokens):
            key = tokens[pos]
            pos += 1
            if key == '}':
                if not nested:
                    raise ValueError('Unexpected closing brace')
                return result
            if pos >= len(tokens) or key == '{':
                raise ValueError('Incomplete VDF entry')
            value = tokens[pos]
            pos += 1
            result[key] = obj(True) if value == '{' else value
        if nested:
            raise ValueError('Incomplete VDF object')
        return result

    return obj()


def read_vdf(path):
    return vdf(path.read_text(encoding='utf-8'))


def atomic_write(path, data):
    if path.exists() and path.read_bytes() == data:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    mode = path.stat().st_mode & 0o777 if path.exists() else 0o644
    fd, name = tempfile.mkstemp(prefix='.' + path.name, dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
        os.chmod(name, mode)
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)
    return True


def desktop(path):
    cfg = configparser.ConfigParser(interpolation=None, strict=False)
    cfg.optionxform = str
    cfg.read(path, encoding='utf-8')
    return cfg


def set_keys(text, values):
    # Only touch keys in the main group. Preserve comments, actions and settings.
    output = []
    in_main = False
    pending = dict(values)
    for line in text.splitlines():
        if line.strip().startswith('['):
            if in_main:
                output.extend(f'{k}={v}' for k, v in pending.items())
                pending.clear()
            in_main = line.strip() == '[Desktop Entry]'
        key = line.split('=', 1)[0] if '=' in line else None
        if in_main and key in values:
            if key in pending:
                output.append(f'{key}={pending.pop(key)}')
            continue
        output.append(line)
    if in_main:
        output.extend(f'{k}={v}' for k, v in pending.items())
    return '\n'.join(output) + '\n'


def installed(roots):
    games = {}
    for root in roots:
        libraries = {root}
        try:
            folders = read_vdf(root / 'steamapps/libraryfolders.vdf')['libraryfolders']
            for value in folders.values():
                if isinstance(value, dict) and 'path' in value:
                    libraries.add(Path(value['path']))
                elif isinstance(value, str) and value.startswith('/'):
                    libraries.add(Path(value))
        except (OSError, ValueError, KeyError):
            pass
        for library in libraries:
            for manifest in (library / 'steamapps').glob('appmanifest_*.acf'):
                try:
                    app = read_vdf(manifest)['AppState']
                    appid, name = app['appid'], app['name']
                    if not appid.isdigit() or TOOLS.match(name):
                        continue
                    # Don't create launchers for partial downloads.
                    if not int(app.get('StateFlags', '0')) & 4:
                        continue
                    games[appid] = (name, root)
                except (OSError, ValueError, KeyError):
                    continue
    return games


def cached_icon(root, appid, icons):
    cache = root / 'appcache/librarycache'
    candidates = list((cache / appid).glob('*.jpg'))
    candidates += list((cache / appid).glob('*.png'))
    candidates += list(cache.glob(f'{appid}_icon.*'))
    # Steam's game icons are small and square. Ignore library artwork.
    import gi
    gi.require_version('GdkPixbuf', '2.0')
    from gi.repository import GdkPixbuf
    for candidate in sorted(candidates):
        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_file(str(candidate))
            if pixbuf.get_width() != pixbuf.get_height() or pixbuf.get_width() > 512:
                continue
            target = icons / f'steam_icon_{appid}.png'
            ok, data = pixbuf.save_to_bufferv('png', [], [])
            if ok:
                atomic_write(target, bytes(data))
                return str(target)
        except Exception as exc:
            print(f'Icon {appid}: {exc}', flush=True)
    return None


def window_classes(games):
    """Recognize native X11 games too, using Steam's process environment."""
    result = {}
    if not os.environ.get('DISPLAY') or not shutil_available('xprop'):
        return result
    try:
        root = subprocess.run(['xprop', '-root', '_NET_CLIENT_LIST'],
                              capture_output=True, text=True, timeout=3).stdout
        for window in re.findall(r'0x[0-9a-fA-F]+', root):
            props = subprocess.run(['xprop', '-id', window, 'WM_CLASS', '_NET_WM_PID', 'WM_TRANSIENT_FOR'],
                                   capture_output=True, text=True, timeout=2).stdout
            pid = re.search(r'_NET_WM_PID\(CARDINAL\) = (\d+)', props)
            cls = re.search(r'WM_CLASS\(STRING\) = "[^"]*", "([^"]+)"', props)
            if not pid or not cls or 'WM_TRANSIENT_FOR(WINDOW)' in props:
                continue
            try:
                env = (Path('/proc') / pid[1] / 'environ').read_bytes().split(b'\0')
            except OSError:
                continue
            appid = next((item[11:].decode() for item in env if item.startswith(b'SteamAppId=')), '')
            name = cls[1]
            if appid in games and name.lower() not in ('steam', 'explorer.exe', 'steamwebhelper'):
                result.setdefault(appid, name)
    except (OSError, subprocess.TimeoutExpired):
        pass
    return result


def shutil_available(command):
    from shutil import which
    return which(command) is not None


def sync(roots, apps, icons, state):
    games = installed(roots)
    observed = window_classes(games)
    existing = {}
    for path in apps.glob('*.desktop'):
        try:
            cfg = desktop(path)
            if 'Desktop Entry' not in cfg:
                continue
            match = GAME_URI.search(cfg['Desktop Entry'].get('Exec', ''))
            if match:
                existing.setdefault(match[1], []).append(path)
        except (OSError, configparser.Error):
            continue
    changed = []
    for appid, (name, root) in sorted(games.items(), key=lambda item: int(item[0])):
        icon = cached_icon(root, appid, icons)
        paths = existing.get(appid, [apps / f'steam_app_{appid}.desktop'])
        for path in paths:
            try:
                old = path.read_text(encoding='utf-8') if path.exists() else None
                if old is None:
                    safe_name = name.replace('\\', '\\\\').replace('\n', ' ').replace('\r', ' ')
                    old_text = ('[Desktop Entry]\nType=Application\n'
                                f'Name={safe_name}\nExec=steam steam://rungameid/{appid}\n'
                                'Terminal=false\nCategories=Game;\nX-Steam-Desktop-Sync=true\n')
                else:
                    old_text = old
                cfg = configparser.ConfigParser(interpolation=None, strict=False)
                cfg.optionxform = str
                cfg.read_string(old_text)
                entry = cfg['Desktop Entry']
                values = {}
                # Retain explicit native-game classes and custom user settings.
                current_class = entry.get('StartupWMClass', '')
                if not current_class or current_class == f'steam_app_{appid}':
                    values['StartupWMClass'] = observed.get(appid, current_class or f'steam_app_{appid}')
                if icon:
                    current = entry.get('Icon', '')
                    if (not current or current.startswith('steam_icon_') or
                        '/Steam/appcache/librarycache/' in current or
                        str(icons) in current):
                        values['Icon'] = icon
                elif not entry.get('Icon'):
                    values['Icon'] = 'steam'
                new = set_keys(old_text, values)
                if old != new:
                    if old is not None:
                        backup = state / 'backups' / path.name
                        if not backup.exists():
                            atomic_write(backup, old.encode())
                    if atomic_write(path, new.encode()):
                        changed.append(path.name)
            except (OSError, configparser.Error) as exc:
                print(f'Launcher {path.name}: {exc}', flush=True)
    if changed:
        subprocess.run(['update-desktop-database', str(apps)], check=True)
        print(f'Updated {len(changed)} launchers; {len(games)} installed games.', flush=True)
    return games, changed


def main():
    home = Path.home()
    data = Path(os.environ.get('XDG_DATA_HOME', home / '.local/share'))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--steam-root', type=Path, action='append')
    parser.add_argument('--applications', type=Path, default=data / 'applications')
    parser.add_argument('--icons', type=Path, default=data / 'icons/hicolor/32x32/apps')
    parser.add_argument('--state', type=Path, default=home / '.local/state/steam-desktop-sync')
    args = parser.parse_args()
    candidates = args.steam_root or [data / 'Steam', home / '.steam/steam',
                  home / '.var/app/com.valvesoftware.Steam/.local/share/Steam']
    roots = list(dict.fromkeys(p.resolve() for p in candidates if p.exists()))
    args.state.mkdir(parents=True, exist_ok=True)
    with (args.state / 'sync.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        sync(roots, args.applications, args.icons, args.state)


if __name__ == '__main__':
    main()
