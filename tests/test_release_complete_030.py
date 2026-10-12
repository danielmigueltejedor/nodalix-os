"""Compatibility of published clients and the complete GNOME 0.3.0 payload."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from test_gnome_transition_030 import BRIDGE

ROOT = Path(__file__).resolve().parents[1]


class CompleteReleaseTests(unittest.TestCase):
    def test_actual_historical_updaters_accept_the_bridge_manifest(self):
        definition = json.loads((ROOT / 'release/components.json').read_text())
        manifest = {**definition, 'schema': 1, 'version': '0.3.0'}
        manifest['components'] = [
            {**row, 'asset': row['package'] + '-' + row['version'] + '-2-any.pkg.tar.zst',
             'sha256': '0' * 64, 'size': 1}
            for row in definition['components']
        ]
        tags = subprocess.check_output(['git', 'tag', '--list', '0.2.*'], cwd=ROOT, text=True).splitlines()
        self.assertTrue(tags, 'Historical Git tags are required to validate compatibility')
        with tempfile.TemporaryDirectory() as temp:
            for index, tag in enumerate(tags):
                with self.subTest(version=tag):
                    source = subprocess.check_output(['git', 'show', tag + ':updater/nodalix-updater'], cwd=ROOT)
                    path = Path(temp) / ('client' + str(index) + '.py')
                    path.write_bytes(source)
                    name = 'historical_client_' + str(index)
                    spec = importlib.util.spec_from_file_location(name, path)
                    client = importlib.util.module_from_spec(spec)
                    sys.modules[name] = client
                    spec.loader.exec_module(client)
                    with mock.patch.object(client, 'current_version', return_value=tag), \
                         mock.patch.object(client, 'load_config', return_value={'require_signature': False}), \
                         mock.patch.object(BRIDGE.importlib.util, 'spec_from_loader', return_value=mock.Mock(loader=mock.Mock())), \
                         mock.patch.object(BRIDGE.importlib.util, 'module_from_spec', return_value=client):
                        BRIDGE.verify_bridge_policy(json.dumps(manifest).encode(), manifest)

    def test_every_added_extension_is_packaged_with_real_code(self):
        package = (ROOT / 'packaging/nodalix-gnome/PKGBUILD').read_text()
        for uuid in ('global-menu@unmade.space', 'multi-monitors-bar@frederykabryan',
                     'nodalix-auto-tiling@getnodalia.com'):
            self.assertIn(uuid, package)
            folder = ROOT / 'gnome/extensions' / uuid
            metadata = json.loads((folder / 'metadata.json').read_text())
            self.assertEqual(metadata['uuid'], uuid)
            self.assertIn('51', metadata['shell-version'])
            self.assertTrue((folder / 'extension.js').is_file())
        self.assertTrue((ROOT / 'gnome/lib/global-menu/globalmenu/run.py').is_file())
        self.assertIn('global-menu-launcher', package)

    def test_default_profile_has_no_competing_panel_or_tiling_extensions(self):
        profile = json.loads((ROOT / 'gnome/session/extensions.json').read_text())
        self.assertEqual(len(profile), len(set(profile)))
        self.assertFalse({'nodalix-secondary-bars@getnodalia.com',
                          'multi-monitors-bar@frederykabryan'}.issubset(profile))
        self.assertFalse({'nodalix-auto-tiling@getnodalia.com',
                          'tiling-assistant@leleat-on-github'}.issubset(profile))

    def test_iso_wallpaper_is_owned_only_by_its_package(self):
        tool = (ROOT / 'tools/build-iso.py').read_text()
        self.assertNotIn('live_wallpaper_target', tool)
        self.assertIn('nodalix-wallpapers', tool)

    def test_craft_and_icloud_are_portable_system_integrations(self):
        package = (ROOT / 'packaging/nodalix-integrations/PKGBUILD').read_text()
        self.assertIn('organize-apps', package)
        self.assertIn('icloud-renovar', package)
        self.assertIn('icloud-apple-official.svg', package)
        organizer = (ROOT / 'integrations/scripts/nodalix-organize-apps.py').read_text()
        self.assertIn('ai.storyteller', organizer)
        self.assertIn('cadcraft', organizer)
        desktop = (ROOT / 'integrations/assets/com.nodalix.iCloudAccess.desktop').read_text()
        self.assertIn('Icon=icloud-apple-official', desktop)
        self.assertNotIn('/home/daniel', desktop)
        launcher = (ROOT / 'packaging/nodalix-artcraft-launcher/PKGBUILD').read_text()
        self.assertIn('storytold/craft-launcher', launcher)
        self.assertNotIn('"SKIP"', launcher)


if __name__ == '__main__':
    unittest.main()
