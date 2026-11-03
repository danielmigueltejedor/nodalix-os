"""One-time retirement of duplicate/obsolete Shell extensions with private backups."""
from pathlib import Path
import json
import os
import shutil

RETIRED=('nodalix-shortcuts@getnodalia.com','nodalix-secondary-bars@getnodalia.com')
SHADOWS=('global-menu@unmade.space','nodalix-startup-cursor@local')
DESKTOP_DEFAULTS=('global-menu@unmade.space','rounded-window-corners@ztrahmet.github.io')


def restore_default_extensions(home,settings,system=Path('/usr/share/gnome-shell/extensions')):
    """Restore Nodalix defaults removed by an earlier development candidate."""
    state=Path(os.environ.get('XDG_STATE_HOME',str(home/'.local/state')))
    backup=state/'nodalix/migrations/gnome-0.4.0-default-desktop-extensions'
    backup.mkdir(parents=True,exist_ok=True,mode=0o700)
    saved=backup/'preferences.json'
    if not saved.exists():
        saved.write_text(json.dumps({key:settings.get_strv(key) for key in
            ('enabled-extensions','disabled-extensions')},indent=2)+'\n')
        saved.chmod(0o600)
    defaults=[uuid for uuid in DESKTOP_DEFAULTS if (system/uuid/'metadata.json').is_file()]
    enabled=list(settings.get_strv('enabled-extensions'))
    settings.set_strv('enabled-extensions',enabled+[uuid for uuid in defaults if uuid not in enabled])
    settings.set_strv('disabled-extensions',[uuid for uuid in settings.get_strv('disabled-extensions') if uuid not in defaults])


def restore_tiling_assistant(home,settings,system=Path('/usr/share/gnome-shell/extensions')):
    """Repair the earlier development candidate's retirement, preserving opt-out."""
    uuid='tiling-assistant@leleat-on-github'
    state=Path(os.environ.get('XDG_STATE_HOME',str(home/'.local/state')))
    saved=state/'nodalix/migrations/gnome-0.4.0-native-extensions/preferences.json'
    if not saved.is_file() or not (system/uuid/'metadata.json').is_file():return
    previous=json.loads(saved.read_text())
    enabled=list(settings.get_strv('enabled-extensions'))
    if uuid in previous.get('enabled',[]) and uuid not in enabled and uuid not in settings.get_strv('disabled-extensions'):
        settings.set_strv('enabled-extensions',enabled+[uuid])


def clean_extensions(home,settings,system=Path('/usr/share/gnome-shell/extensions')):
    data=Path(os.environ.get('XDG_DATA_HOME',str(home/'.local/share')))
    state=Path(os.environ.get('XDG_STATE_HOME',str(home/'.local/state')))
    backup=state/'nodalix/migrations/gnome-0.4.0-native-extensions'
    backup.mkdir(parents=True,exist_ok=True,mode=0o700)
    saved=backup/'preferences.json'
    enabled=list(settings.get_strv('enabled-extensions'))
    disabled=list(settings.get_strv('disabled-extensions'))
    if not saved.exists():saved.write_text(json.dumps({'enabled':enabled,'disabled':disabled},indent=2))
    for uuid in (*RETIRED,*SHADOWS):
        folder=data/'gnome-shell/extensions'/uuid
        if not folder.exists() or folder.is_symlink():continue
        if uuid in SHADOWS and not (system/uuid/'metadata.json').is_file():continue
        target=backup/'extensions'/uuid
        if target.exists():raise RuntimeError('Ya existe una copia de seguridad distinta de '+uuid)
        target.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
        shutil.move(str(folder),str(target))
    # Keep the existing native GNOME multi-monitor provider and user preferences.
    desired=[uuid for uuid in enabled if uuid not in RETIRED and uuid!='nodalix-login-background@getnodalia.com']
    if 'nodalix-secondary-bars@getnodalia.com' in enabled and 'multi-monitors-bar@frederykabryan' not in desired:
        desired.append('multi-monitors-bar@frederykabryan')
    if 'nodalix-auto-tiling@getnodalia.com' not in desired:desired.append('nodalix-auto-tiling@getnodalia.com')
    settings.set_strv('enabled-extensions',desired)
    settings.set_strv('disabled-extensions',[uuid for uuid in disabled if uuid not in RETIRED])


def restore_native_panels(home,settings):
    """Retire the development-only replacement and use the native panel mirror."""
    enabled=list(settings.get_strv('enabled-extensions'))
    old='nodalix-secondary-bars@getnodalia.com'
    native='multi-monitors-bar@frederykabryan'
    if old in enabled:
        enabled=[item for item in enabled if item!=old]
        if native not in enabled:enabled.append(native)
        settings.set_strv('enabled-extensions',enabled)
    disabled=settings.get_strv('disabled-extensions')
    if old in disabled:settings.set_strv('disabled-extensions',[item for item in disabled if item!=old])


def restore_stock_appearance(home,settings):
    """Drop development-only visual defaults; leave useful desktop features."""
    from gi.repository import Gio
    state=Path(os.environ.get('XDG_STATE_HOME',str(home/'.local/state')))
    backup=state/'nodalix/migrations/gnome-0.4.0-stock-appearance'
    backup.mkdir(parents=True,exist_ok=True,mode=0o700)
    saved=backup/'preferences.json'
    if not saved.exists():
        saved.write_text(json.dumps({'enabled':settings.get_strv('enabled-extensions'),
            'disabled':settings.get_strv('disabled-extensions')},indent=2)+'\n')
        saved.chmod(0o600)
    folder=Path('/usr/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas')
    if not (folder/'gschemas.compiled').is_file():return
    source=Gio.SettingsSchemaSource.new_from_directory(str(folder),Gio.SettingsSchemaSource.get_default(),False)
    for suffix in ('panel','popup','dash-to-dock','overview'):
        schema=source.lookup('org.gnome.shell.extensions.blur-my-shell.'+suffix,False)
        if not schema:continue
        prefs=Gio.Settings.new_full(schema,None,None)
        # Undo only the candidate's brightness preset, retaining custom values.
        if schema.has_key('brightness') and abs(prefs.get_double('brightness')-0.7)<1e-9:prefs.reset('brightness')
        if suffix=='popup' and schema.has_key('static-blur'):prefs.reset('static-blur')
