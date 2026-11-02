"""One-time retirement of duplicate/obsolete Shell extensions with private backups."""
from pathlib import Path
import json
import os
import shutil

RETIRED=('nodalix-shortcuts@getnodalia.com','multi-monitors-bar@frederykabryan')
SHADOWS=('global-menu@unmade.space','nodalix-startup-cursor@local')


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
    # One provider for each feature; preferences for the surviving provider stay.
    desired=[uuid for uuid in enabled if uuid not in RETIRED and uuid!='nodalix-login-background@getnodalia.com']
    if 'multi-monitors-bar@frederykabryan' in enabled and 'nodalix-secondary-bars@getnodalia.com' not in desired:
        desired.append('nodalix-secondary-bars@getnodalia.com')
    if 'nodalix-auto-tiling@getnodalia.com' not in desired:desired.append('nodalix-auto-tiling@getnodalia.com')
    settings.set_strv('enabled-extensions',desired)
    settings.set_strv('disabled-extensions',[uuid for uuid in disabled if uuid not in RETIRED])


def apply_cached_blur():
    from gi.repository import Gio
    source=Gio.SettingsSchemaSource.get_default()
    folder=Path('/usr/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas')
    if not (folder/'gschemas.compiled').is_file():return
    local=Gio.SettingsSchemaSource.new_from_directory(str(folder),source,False)
    for name in ('panel','popup'):
        schema=local.lookup('org.gnome.shell.extensions.blur-my-shell.'+name,False)
        if not schema:continue
        prefs=Gio.Settings.new_full(schema,None,None)
        if prefs.get_string('pipeline') in ('pipeline_default','pipeline_default_rounded'):
            prefs.set_boolean('static-blur',True)
