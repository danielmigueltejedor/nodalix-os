"""Migrate known broken defaults without removing custom input configuration."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from test_gnome_transition_030 import SYSTEM

ROOT=Path(__file__).resolve().parents[1]


class KeyboardMigrationTests(unittest.TestCase):
    def test_default_launch_bindings_and_explicit_user_choices_survive_migration(self):
        with tempfile.TemporaryDirectory() as directory:
            script=f'''import sys
sys.path.insert(0,{str(ROOT/'gnome/migration')!r})
import user_preferences as p
from gi.repository import Gio
p.desktop_available=lambda desktop:False
p.apply_preferences()
schema='org.gnome.settings-daemon.plugins.media-keys.custom-keybinding'
base='/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/'
expected=[('terminal','<Super>Return','xdg-terminal-exec'),('browser','<Super>b','nodalix-open-default browser'),('mail','<Super>m','nodalix-open-default mail')]
for name,key,command in expected:
    s=Gio.Settings.new_with_path(schema,base+'nodalix-'+name+'/')
    assert s.get_string('binding')==key
    assert s.get_string('command')==command
mail=Gio.Settings.new_with_path(schema,base+'nodalix-mail/')
mail.set_string('binding','<Alt>m');mail.set_string('command','personal-mail')
focus=Gio.Settings.new('org.gnome.desktop.wm.preferences');focus.set_string('focus-mode','click')
p.apply_preferences()
assert mail.get_string('binding')=='<Alt>m'
assert mail.get_string('command')=='personal-mail'
assert focus.get_string('focus-mode')=='click'
'''
            subprocess.run(['/usr/bin/python3','-c',script],check=True,capture_output=True,
                env={**os.environ,'GSETTINGS_BACKEND':'memory','HOME':directory,
                     'XDG_CONFIG_HOME':directory+'/config','XDG_DATA_HOME':directory+'/data','XDG_STATE_HOME':directory+'/state'})

    def test_launch_helpers_use_configured_browser_and_mail_application(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);apps=root/'data/applications';apps.mkdir(parents=True)
            config=root/'config';config.mkdir()
            launch=root/'launch.py';launch.write_text('import sys\nfrom pathlib import Path\nPath(sys.argv[1]).write_text(" ".join(sys.argv[2:]))\n')
            for kind,mime in (('browser','x-scheme-handler/https'),('mail','x-scheme-handler/mailto')):
                (apps/(kind+'.desktop')).write_text('[Desktop Entry]\nType=Application\nName=QA '+kind+'\nExec=/usr/bin/python3 '+str(launch)+' '+str(root/(kind+'-opened'))+' %u\nMimeType='+mime+';\n')
            script=f'''import importlib.util,os,subprocess,time
from pathlib import Path
from gi.repository import Gio
for kind,mime in [('browser','x-scheme-handler/https'),('mail','x-scheme-handler/mailto')]:
    app=Gio.DesktopAppInfo.new(kind+'.desktop')
    assert app.set_as_default_for_type(mime)
    subprocess.run(['/usr/bin/python3',{str(ROOT/'gnome/session/nodalix-open-default')!r},kind],check=True)
    for _ in range(100):
        if Path({directory!r},kind+'-opened').exists():break
        time.sleep(.02)
    else:raise AssertionError(kind+' did not launch')
assert Path({directory!r},'mail-opened').read_text()=='mailto:'
'''
            subprocess.run(['dbus-run-session','--','/usr/bin/python3','-c',script],check=True,capture_output=True,
                env={**os.environ,'XDG_DATA_HOME':str(root/'data'),'XDG_CONFIG_HOME':str(config)})

    def test_exact_legacy_remap_is_backed_up_and_retired_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/'etc/keyd/default.conf';path.parent.mkdir(parents=True)
            original='[ids]\n1234:5678:abcdef01\n1234:5678:abcdef02\n\n[meta]\nc = C-c\nv = C-v\nz = C-x\nq = C-w\n'
            path.write_text(original)
            self.assertTrue(SYSTEM.retire_legacy_keyboard_remap(root))
            self.assertFalse(path.exists())
            saved=root/'var/lib/nodalix-updater/migrations/gnome-0.4.0/files/etc/keyd/default.conf'
            self.assertEqual(saved.read_text(),original)
            self.assertFalse(SYSTEM.retire_legacy_keyboard_remap(root))

    def test_custom_remapping_and_symlinks_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/'etc/keyd/default.conf';path.parent.mkdir(parents=True)
            custom='[ids]\n1234:5678:abcdef01\n[meta]\nc = C-c\nv = C-v\nz = C-z\n'
            path.write_text(custom)
            self.assertFalse(SYSTEM.retire_legacy_keyboard_remap(root));self.assertEqual(path.read_text(),custom)
            target=path.with_name('personal.conf');path.rename(target);path.symlink_to(target)
            self.assertFalse(SYSTEM.retire_legacy_keyboard_remap(root));self.assertTrue(path.is_symlink())

    def test_screenshot_dock_conflict_is_removed_without_changing_custom_shortcut(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            source=ROOT/'gnome/extensions/dash-to-dock@micxgx.gmail.com/schemas/org.gnome.shell.extensions.dash-to-dock.gschema.xml'
            shutil.copy2(source,root/source.name)
            subprocess.run(['glib-compile-schemas',directory],check=True,capture_output=True)
            script=f'''import sys
sys.path.insert(0,{str(ROOT/'gnome/migration')!r})
from user_preferences import reserve_default_shortcuts
from gi.repository import Gio
s=Gio.Settings.new('org.gnome.shell.extensions.dash-to-dock')
s.set_strv('shortcut',['<Super>q'])
s.set_string('shortcut-text','<Super>q')
s.set_strv('app-shift-hotkey-3',['<Shift><Super>3'])
s.set_strv('app-shift-hotkey-4',['<Alt>F8'])
reserve_default_shortcuts();reserve_default_shortcuts()
assert s.get_strv('shortcut')==[]
assert s.get_string('shortcut-text')==''
assert s.get_strv('app-shift-hotkey-3')==[]
assert s.get_strv('app-shift-hotkey-4')==['<Alt>F8']
'''
            subprocess.run(['/usr/bin/python3','-c',script],check=True,capture_output=True,
                env={**os.environ,'GSETTINGS_SCHEMA_DIR':directory,'GSETTINGS_BACKEND':'memory'})


if __name__=='__main__':unittest.main()
