"""Duplicate-extension retirement retains recoverable copies and user settings."""
import importlib.util
import os
from pathlib import Path
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
    def test_duplicate_sources_are_backed_up_and_only_one_provider_remains(self):
        with tempfile.TemporaryDirectory() as directory:
            home=Path(directory);data=home/'data';state=home/'state';system=home/'system'
            uuids=[*cleanup.RETIRED,*cleanup.SHADOWS,'my-own-extension@example.invalid']
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
            self.assertIn('nodalix-secondary-bars@getnodalia.com',prefs.values['enabled-extensions'])
            self.assertIn('my-own-extension@example.invalid',prefs.values['enabled-extensions'])
            self.assertEqual(prefs.values['disabled-extensions'],['nodalix-connect@getnodalia.com'])

    def test_retiring_the_last_keyd_configuration_stops_instead_of_hot_reloading(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);folder=root/'etc/keyd';folder.mkdir(parents=True)
            runner=mock.Mock()
            SYSTEM.refresh_keyboard_remapper(root,runner)
            self.assertEqual(runner.call_args.args[0],['systemctl','stop','keyd.service'])
            (folder/'custom.conf').write_text('[ids]\n*\n')
            SYSTEM.refresh_keyboard_remapper(root,runner)
            self.assertEqual(runner.call_args.args[0],['systemctl','restart','keyd.service'])

if __name__=='__main__':unittest.main()
