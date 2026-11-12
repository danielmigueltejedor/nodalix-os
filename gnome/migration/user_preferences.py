"""GNOME preferences that preserve explicit user choices on upgrades."""
import configparser
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from gi.repository import Gio, GLib


def desktop_available(desktop):
    # PyGObject versions differ: a missing desktop can return None or raise.
    try:
        return Gio.DesktopAppInfo.new(desktop) is not None
    except TypeError:
        return False


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


def reserve_default_shortcuts():
    source=Gio.SettingsSchemaSource.get_default()
    schema=source.lookup('org.gnome.shell.extensions.dash-to-dock',True)
    if not schema:
        for root in (Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share'))),Path('/usr/share')):
            folder=root/'gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas'
            if (folder/'gschemas.compiled').is_file():
                local=Gio.SettingsSchemaSource.new_from_directory(str(folder),source,False)
                schema=local.lookup('org.gnome.shell.extensions.dash-to-dock',False)
                if schema:break
    if not schema:return
    settings=Gio.Settings.new_full(schema,None,None)
    if settings.get_string('shortcut-text')=='<Super>q':settings.set_string('shortcut-text','')
    previous=settings.get_strv('shortcut')
    remaining=[value for value in previous if value!='<Super>q']
    if remaining!=previous:settings.set_strv('shortcut',remaining)
    for number in (3,4):
        key=f'app-shift-hotkey-{number}'
        previous=settings.get_strv(key)
        factory={f'<Shift><Super>{number}',f'<Super><Shift>{number}'}
        remaining=[value for value in previous if value not in factory]
        if remaining!=previous:settings.set_strv(key,remaining)


def clean_ghostty(path):
    if not path.is_file() or path.is_symlink():
        return
    managed = {'window-decoration':'client', 'gtk-titlebar':'true', 'gtk-titlebar-style':'tabs',
               'gtk-tabs-location':'top', 'gtk-wide-tabs':'true'}
    # Retire duplicate compositor-specific values while retaining unrelated settings.
    original = path.read_text()
    lines = original.splitlines()
    native={}
    for line in lines:
        match=re.match(r'^\s*keybind\s*=\s*(performable:)?(ctrl\+insert|shift\+insert)=(.*)',line)
        if match:native[match[2]]=line
    lines=[line for line in lines if not re.match(r'^\s*keybind\s*=\s*(performable:)?(ctrl\+insert|shift\+insert)=',line)]
    output = [line for line in lines if line.split('=', 1)[0].strip() not in managed
              and not re.match(r'^\s*keybind\s*=\s*(performable:)?ctrl\+[cv]=', line)]
    output += [f'{key} = {value}' for key, value in managed.items()]
    output += ['keybind = performable:ctrl+c=copy_to_clipboard', 'keybind = ctrl+v=paste_from_clipboard']
    # Dedicated native clipboard commands use terminal-safe Insert chords.
    for key,action in [('ctrl+insert','copy_to_clipboard'),('shift+insert','paste_from_clipboard')]:
        output.append(native.get(key,f'keybind = {key}={action}'))
    text = '\n'.join(output)+'\n'
    if text == original:
        return

    backup = path.with_name(path.name + '.pre-gnome-030')
    try:
        with backup.open('x') as stream:
            stream.write(original)
    except FileExistsError:
        pass

    with tempfile.NamedTemporaryFile(mode='w', dir=path.parent, delete=False) as stream:
        staged = Path(stream.name)
        try:
            stream.write(text)
            stream.flush()
            shutil.copymode(path, staged)
            if not path.is_symlink() and path.read_text() == original:
                staged.replace(path)
        finally:
            staged.unlink(missing_ok=True)


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


LEGACY_TERMINAL_CARD = b'''# Show the Nodalix/Arch system card only for real interactive terminals.
if status is-interactive
    if type -q fastfetch
        # Select a layout from the terminal's initial geometry. This keeps the
        # logo and data aligned in tiled, floating and very small terminals.
        set -l ff_dir "$HOME/.config/fastfetch"
        if test "$COLUMNS" -ge 108; and test "$LINES" -ge 20
            fastfetch --config "$ff_dir/config.jsonc"
        else if test "$COLUMNS" -ge 72; and test "$LINES" -ge 11
            fastfetch --config "$ff_dir/compact.jsonc"
        else
            fastfetch --config "$ff_dir/minimal.jsonc"
        end
    end
end

'''


def replace_terminal_default(path, old, new=b''):
    if path.is_symlink() or not path.is_file():
        return
    original = path.read_bytes()
    if original.count(old) != 1:
        return
    backup = path.with_name(path.name+'.pre-nodalix-terminal')
    if not backup.exists():
        shutil.copy2(path, backup, follow_symlinks=False)
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as updated:
        staged = Path(updated.name)
        try:
            updated.write(original.replace(old, new, 1))
            updated.flush()
            shutil.copymode(path, staged)
            if not path.is_symlink() and path.read_bytes() == original:
                staged.replace(path)
        finally:
            staged.unlink(missing_ok=True)


def retire_legacy_terminal_card(config_home):
    """Remove the known Arch greeting and prompt emblem."""
    replace_terminal_default(config_home/'fish/config.fish', LEGACY_TERMINAL_CARD)
    prompt = b'format = """\n$directory$git_branch$git_status$character"""'
    legacy_prompt = prompt.replace(b'\n', '\n[](bold #1793d1) '.encode(), 1)
    replace_terminal_default(config_home/'starship.toml', legacy_prompt, prompt)


def configure_terminal_welcome(home, shell):
    """Add the managed hook once and preserve personal startup files."""
    startup = {'bash': '.bashrc', 'zsh': '.zshrc'}.get(Path(shell).name)
    if startup is None:
        return
    path = home/startup
    hook = b'. /usr/lib/nodalix/nodalix-welcome.sh'
    if path.is_symlink() or (path.exists() and not path.is_file()):
        return
    if path.is_file() and hook in path.read_bytes():
        return
    content = b'\n# Nodalix terminal welcome\n[ ! -r /usr/lib/nodalix/nodalix-welcome.sh ] || ' + hook + b'\n'
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'ab') as output:
        output.write(content)


def apply_preferences():
    reserve_default_shortcuts()
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
                                    ('mail','nodalix-open-default mail','<Super>m'),
                                    ('copy','nodalix-clipboard copy','<Super>c'),
                                    ('paste','nodalix-clipboard paste','<Super>v')]:
        location = f'/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/nodalix-{name}/'
        if location not in bindings and binding in occupied:
            continue
        custom = Gio.Settings.new_with_path('org.gnome.settings-daemon.plugins.media-keys.custom-keybinding', location)
        # Keep both unrelated bindings and explicit changes to Nodalix bindings.
        for key,value in [('name',{'copy':'Copiar','paste':'Pegar'}.get(name,'Nodalix '+name)),('command',command),('binding',binding)]:
            if custom.get_user_value(key) is None:custom.set_string(key,value)
        if location not in bindings:
            bindings.append(location)
    media.set_strv('custom-keybindings', bindings)
    config = Path(os.environ.get('XDG_CONFIG_HOME', str(Path.home()/'.config')))
    terminal = config/'xdg-terminals.list'
    if not terminal.exists() and desktop_available('com.mitchellh.ghostty.desktop'):
        terminal.parent.mkdir(parents=True, exist_ok=True)
        terminal.write_text('com.mitchellh.ghostty.desktop\n')
    ghostty=config/'ghostty/config'
    if not ghostty.exists() and desktop_available('com.mitchellh.ghostty.desktop'):
        ghostty.parent.mkdir(parents=True,exist_ok=True);ghostty.touch(mode=0o600)
    clean_ghostty(ghostty)
    data = Path(os.environ.get('XDG_DATA_HOME', str(Path.home()/'.local/share')))
    state = Path(os.environ.get('XDG_STATE_HOME', str(Path.home()/'.local/state')))
    retire_chatgpt(data/'applications/chatgpt.desktop', state/'nodalix/migrations/gnome-first-0.3.0-v2/chatgpt.desktop')
    Gio.Settings.sync()
