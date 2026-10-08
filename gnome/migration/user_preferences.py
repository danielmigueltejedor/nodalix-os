"""GNOME preferences that preserve explicit user choices on upgrades."""
import configparser
import os
from pathlib import Path
import re
import shutil
import subprocess
from gi.repository import Gio, GLib


def layout_from_system(root=Path('/')):
    # localed exports the selected XKB layout; locale is only a fallback.
    conf = root/'etc/X11/xorg.conf.d/00-keyboard.conf'
    text = conf.read_text() if conf.is_file() else ''
    layouts = re.search(r'Option\s+"XkbLayout"\s+"([^"]+)"', text, re.I)
    variants = re.search(r'Option\s+"XkbVariant"\s+"([^"]*)"', text, re.I)
    if layouts:
        variants = variants.group(1).split(',') if variants else []
        return [('xkb', value + ('+'+variants[i] if i < len(variants) and variants[i] else ''))
                for i, value in enumerate(layouts.group(1).split(',')) if value]
    vconsole = root/'etc/vconsole.conf'
    keymap = re.search(r'^KEYMAP=["\']?([^"\'\s]+)', vconsole.read_text(), re.M) if vconsole.is_file() else None
    mapping = {'es': 'es', 'us': 'us', 'uk': 'gb', 'de': 'de', 'fr': 'fr'}
    if keymap and keymap.group(1) in mapping:
        return [('xkb', mapping[keymap.group(1)])]
    locale = root/'etc/locale.conf'
    text = locale.read_text() if locale.is_file() else ''
    return [('xkb', 'es' if re.search(r'LANG=["\']?es_', text) else 'us')]


def set_unset(schema, values):
    prefs = Gio.Settings.new(schema)
    for key, value in values.items():
        if prefs.get_user_value(key) is None:
            prefs.set_value(key, GLib.Variant.parse(None, value, None, None))


def clean_ghostty(path):
    if not path.is_file() or path.is_symlink():
        return
    managed = {'window-decoration':'client', 'gtk-titlebar':'true', 'gtk-titlebar-style':'tabs',
               'gtk-tabs-location':'top', 'gtk-wide-tabs':'true'}
    # Retire duplicate compositor-specific values while retaining unrelated settings.
    lines = path.read_text().splitlines()
    output = [line for line in lines if line.split('=', 1)[0].strip() not in managed
              and not re.match(r'^\s*keybind\s*=\s*(performable:)?ctrl\+[cv]=', line)]
    output += [f'{key} = {value}' for key, value in managed.items()]
    output += ['keybind = performable:ctrl+c=copy_to_clipboard', 'keybind = ctrl+v=paste_from_clipboard']
    text = '\n'.join(output)+'\n'
    if text != path.read_text():
        backup = path.with_name(path.name+'.pre-gnome-030')
        if not backup.exists():
            shutil.copy2(path, backup)
        path.write_text(text)


def retire_chatgpt(path, backup):
    if not path.is_file() or path.is_symlink():
        return False
    execs = [line.removeprefix('Exec=').strip() for line in path.read_text().splitlines() if line.startswith('Exec=')]
    old = str(Path.home()/'.config/quickshell/scripts/nodalix-sync-user-language.sh')
    if execs not in [[prefix+' --launch-chatgpt'] for prefix in (old,'~/.config/quickshell/scripts/nodalix-sync-user-language.sh','$HOME/.config/quickshell/scripts/nodalix-sync-user-language.sh')]:
        return False
    backup.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if not backup.exists():
        path.replace(backup)
        return True
    return False


def apply_preferences():
    prefs = Gio.Settings.new('org.gnome.desktop.input-sources')
    if prefs.get_user_value('sources') is None:
        prefs.set_value('sources', GLib.Variant('a(ss)', layout_from_system()))
    set_unset('org.gnome.desktop.wm.preferences', {'focus-mode':"'sloppy'", 'auto-raise':'false', 'button-layout':"':minimize,maximize,close'"})
    set_unset('org.gnome.desktop.wm.keybindings', {'close':"['<Super>q']", 'toggle-fullscreen':"['<Super>f']"})
    shell = Gio.Settings.new('org.gnome.shell.keybindings')
    # GNOME's built-in message-tray keys conflict with mail and paste.
    current = shell.get_strv('toggle-message-tray')
    if shell.get_user_value('toggle-message-tray') is None or current in (['<Super>v','<Super>m'], ['<Super>m','<Super>v']):
        shell.set_strv('toggle-message-tray', ['<Super>n'])
    set_unset('org.gnome.shell.keybindings', {'show-screenshot-ui':"['<Super><Shift>4']", 'screenshot':"['<Super><Shift>3']"})
    media = Gio.Settings.new('org.gnome.settings-daemon.plugins.media-keys')
    bindings = list(media.get_strv('custom-keybindings'))
    occupied={Gio.Settings.new_with_path('org.gnome.settings-daemon.plugins.media-keys.custom-keybinding', path).get_string('binding') for path in bindings}
    for name, command, binding in [('terminal','xdg-terminal-exec','<Super>Return'),
                                    ('browser','nodalix-open-default browser','<Super>b'),
                                    ('mail','nodalix-open-default mail','<Super>m')]:
        location = f'/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/nodalix-{name}/'
        if location not in bindings and binding in occupied:
            continue
        custom = Gio.Settings.new_with_path('org.gnome.settings-daemon.plugins.media-keys.custom-keybinding', location)
        # Keep both unrelated bindings and explicit changes to Nodalix bindings.
        for key,value in [('name','Nodalix '+name),('command',command),('binding',binding)]:
            if custom.get_user_value(key) is None:custom.set_string(key,value)
        if location not in bindings:
            bindings.append(location)
    media.set_strv('custom-keybindings', bindings)
    config = Path(os.environ.get('XDG_CONFIG_HOME', str(Path.home()/'.config')))
    terminal = config/'xdg-terminals.list'
    if not terminal.exists() and Gio.DesktopAppInfo.new('com.mitchellh.ghostty.desktop'):
        terminal.parent.mkdir(parents=True, exist_ok=True)
        terminal.write_text('com.mitchellh.ghostty.desktop\n')
    ghostty=config/'ghostty/config'
    if not ghostty.exists() and Gio.DesktopAppInfo.new('com.mitchellh.ghostty.desktop'):
        ghostty.parent.mkdir(parents=True,exist_ok=True);ghostty.touch(mode=0o600)
    clean_ghostty(ghostty)
    data = Path(os.environ.get('XDG_DATA_HOME', str(Path.home()/'.local/share')))
    state = Path(os.environ.get('XDG_STATE_HOME', str(Path.home()/'.local/state')))
    retire_chatgpt(data/'applications/chatgpt.desktop', state/'nodalix/migrations/gnome-first-0.3.0-v2/chatgpt.desktop')
    Gio.Settings.sync()
