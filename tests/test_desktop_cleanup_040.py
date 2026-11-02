"""Duplicate-extension retirement retains recoverable copies and user settings."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from test_gnome_transition_030 import SYSTEM

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('desktop_040',ROOT/'gnome/migration/desktop_040.py')
cleanup=importlib.util.module_from_spec(spec);spec.loader.exec_module(cleanup)

class Settings:
    def __init__(self,enabled):self.values={'enabled-extensions':enabled,'disabled-extensions':['nodalix-connect@getnodalia.com']}
    def get_strv(self,key):return self.values[key][:]
    def set_strv(self,key,value):self.values[key]=value

class DesktopCleanup(unittest.TestCase):
    def test_stock_appearance_keeps_blur_tiling_and_dock(self):
        code='''
import json,runpy
from pathlib import Path
from gi.repository import Gio
m=runpy.run_path(SOURCE);home=Path.home()
p=Gio.Settings.new('org.gnome.shell')
keep=['blur-my-shell@aunetx','multi-monitors-bar@frederykabryan',
      'tiling-assistant@leleat-on-github','dash-to-dock@micxgx.gmail.com']
original=keep+['global-menu@unmade.space','rounded-window-corners@ztrahmet.github.io']
p.set_strv('enabled-extensions',original)
m['restore_stock_appearance'](home,p);m['restore_stock_appearance'](home,p)
assert p.get_strv('enabled-extensions')==keep
saved=home/'.local/state/nodalix/migrations/gnome-0.4.0-stock-appearance/preferences.json'
assert json.loads(saved.read_text())['enabled']==original
assert saved.stat().st_mode & 0o777==0o600
'''
        with tempfile.TemporaryDirectory() as directory:
            result=subprocess.run([sys.executable,'-c','SOURCE='+repr(str(ROOT/'gnome/migration/desktop_040.py'))+'\n'+code],
                env=dict(os.environ,HOME=directory,XDG_STATE_HOME=directory+'/.local/state',GSETTINGS_BACKEND='memory'),
                capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)

    def test_duplicate_sources_are_backed_up_and_only_one_provider_remains(self):
        with tempfile.TemporaryDirectory() as directory:
            home=Path(directory);data=home/'data';state=home/'state';system=home/'system'
            uuids=[*cleanup.RETIRED,*cleanup.SHADOWS,'tiling-assistant@leleat-on-github','my-own-extension@example.invalid']
            for uuid in uuids:
                folder=data/'gnome-shell/extensions'/uuid;folder.mkdir(parents=True);(folder/'extension.js').write_text('personal snapshot')
                target=system/uuid;target.mkdir(parents=True);(target/'metadata.json').write_text('{}')
            prefs=Settings(uuids)
            with mock.patch.dict(os.environ,{'XDG_DATA_HOME':str(data),'XDG_STATE_HOME':str(state)}):
                cleanup.clean_extensions(home,prefs,system);cleanup.clean_extensions(home,prefs,system)
            backup=state/'nodalix/migrations/gnome-0.4.0-native-extensions/extensions'
            for uuid in (*cleanup.RETIRED,*cleanup.SHADOWS):
                self.assertEqual((backup/uuid/'extension.js').read_text(),'personal snapshot')
                self.assertNotIn(uuid,prefs.values['enabled-extensions']) if uuid in cleanup.RETIRED else None
            self.assertIn('multi-monitors-bar@frederykabryan',prefs.values['enabled-extensions'])
            self.assertIn('my-own-extension@example.invalid',prefs.values['enabled-extensions'])
            self.assertIn('tiling-assistant@leleat-on-github',prefs.values['enabled-extensions'])
            self.assertTrue((data/'gnome-shell/extensions/tiling-assistant@leleat-on-github/extension.js').is_file())
            self.assertEqual(prefs.values['disabled-extensions'],['nodalix-connect@getnodalia.com'])

    def test_native_panel_migration_is_idempotent_and_preserves_other_extensions(self):
        old='nodalix-secondary-bars@getnodalia.com'
        native='multi-monitors-bar@frederykabryan'
        prefs=Settings([old,'blur-my-shell@aunetx','tiling-assistant@leleat-on-github'])
        cleanup.restore_native_panels(Path('/unused'),prefs)
        cleanup.restore_native_panels(Path('/unused'),prefs)
        self.assertEqual(prefs.get_strv('enabled-extensions'),
            ['blur-my-shell@aunetx','tiling-assistant@leleat-on-github',native])
        prefs=Settings(['my-extension'])
        cleanup.restore_native_panels(Path('/unused'),prefs)
        self.assertEqual(prefs.get_strv('enabled-extensions'),['my-extension'])

    def test_retiring_the_last_keyd_configuration_stops_instead_of_hot_reloading(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);folder=root/'etc/keyd';folder.mkdir(parents=True)
            runner=mock.Mock()
            SYSTEM.refresh_keyboard_remapper(root,runner)
            self.assertEqual(runner.call_args.args[0],['systemctl','stop','keyd.service'])
            (folder/'custom.conf').write_text('[ids]\n*\n')
            SYSTEM.refresh_keyboard_remapper(root,runner)
            self.assertEqual(runner.call_args.args[0],['systemctl','restart','keyd.service'])

    def test_development_candidate_restores_tiling_assistant_without_overriding_disable(self):
        with tempfile.TemporaryDirectory() as directory:
            home=Path(directory);state=home/'state';system=home/'system';uuid='tiling-assistant@leleat-on-github'
            saved=state/'nodalix/migrations/gnome-0.4.0-native-extensions/preferences.json'
            saved.parent.mkdir(parents=True);saved.write_text(json.dumps({'enabled':[uuid]}))
            folder=system/uuid;folder.mkdir(parents=True);(folder/'metadata.json').write_text('{}')
            prefs=Settings([])
            with mock.patch.dict(os.environ,{'XDG_STATE_HOME':str(state)}):
                cleanup.restore_tiling_assistant(home,prefs,system)
                cleanup.restore_tiling_assistant(home,prefs,system)
                self.assertEqual(prefs.get_strv('enabled-extensions'),[uuid])
                prefs.values['enabled-extensions']=[];prefs.values['disabled-extensions']=[uuid]
                cleanup.restore_tiling_assistant(home,prefs,system)
                self.assertEqual(prefs.get_strv('enabled-extensions'),[])

if __name__=='__main__':unittest.main()
