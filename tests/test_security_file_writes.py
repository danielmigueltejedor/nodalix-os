"""Regression checks for files managed by migration, integrations and ISO tooling."""
import importlib.util
import os
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
import types
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]


def load_script(path):
    spec = importlib.util.spec_from_file_location('script_under_test', ROOT / path)
    module = importlib.util.module_from_spec(spec)
    repository = types.ModuleType('gi.repository')
    for namespace in ('Gio', 'GLib', 'ECal', 'EDataServer'):
        setattr(repository, namespace, object())
    gi = types.ModuleType('gi')
    gi.repository = repository
    gi.require_version = lambda *_: None
    stubs = {'gi': gi, 'gi.repository': repository}
    for name in ('dbus', 'dbus.exceptions', 'iphonebridge.bus', 'iphonebridge.hfp.events'):
        stubs[name] = types.ModuleType(name)
    stubs['dbus'].exceptions = stubs['dbus.exceptions']
    stubs['iphonebridge.bus'].system_bus = object()
    stubs['iphonebridge.hfp.events'].CallEvent = object
    stubs['iphonebridge.hfp.events'].call_event_from_ofono = object()
    with patch.dict(sys.modules, stubs):
        spec.loader.exec_module(module)
    return module


class SecurityFileWrites(unittest.TestCase):
    def setUp(self):
        scratch = TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)

    def test_ghostty_migration_keeps_existing_backup_and_custom_settings(self):
        migration = load_script('gnome/migration/user_preferences.py')
        config = self.root / 'config'
        config.write_text('font-size = 12\n')
        backup = config.with_name('config.pre-gnome-030')
        backup.write_text('previous backup')
        migration.clean_ghostty(config)
        self.assertIn('font-size = 12', config.read_text())
        self.assertIn('window-decoration = client', config.read_text())
        self.assertEqual(backup.read_text(), 'previous backup')

    def test_app_icon_state_replaces_the_directory_entry(self):
        icons = load_script('integrations/scripts/nodalix-app-icons.py')
        with patch.dict(os.environ, {
            'XDG_DATA_HOME': str(self.root / 'data'),
            'XDG_STATE_HOME': str(self.root / 'state'),
        }):
            folder = self.root / 'state/nodalix/icons'
            folder.mkdir(parents=True)
            original = self.root / 'original'
            original.write_text('original')
            resolved = folder / 'resolved.json'
            resolved.symlink_to(original)
            icons.synchronize(self.root, data_roots=[], theme_roots=[self.root])
        self.assertFalse(resolved.is_symlink())
        self.assertEqual(resolved.read_text(), '{}\n')
        self.assertEqual(original.read_text(), 'original')

    def test_calendar_accepts_only_writable_ics_destinations(self):
        calendar = load_script('integrations/scripts/nodalix-calendar-bridge.py')
        family = self.root / 'family'
        family.mkdir()
        with patch.object(calendar, 'ROOT', self.root):
            self.assertEqual(calendar.pick_rel('uid', None, None, family),
                             calendar.default_rel('uid', family))
            with self.assertRaisesRegex(ValueError, 'writable collection'):
                calendar.write_group('uid', 'family/config.txt', {}, family)

    def test_wireplumber_backup_preserves_previous_config(self):
        ofono = load_script('phone-link/src/iphonebridge/hfp/ofono_client.py')
        config = self.root / '51-bluez-hfp-hf.conf'
        config.write_text('previous configuration')
        with patch.object(ofono, 'WIREPLUMBER_HFP_CONF', config):
            written, backup = ofono.write_wireplumber_config()
            self.assertEqual(written, config)
            self.assertEqual(backup.read_text(), 'previous configuration')
            self.assertEqual(config.read_text(), ofono._WP_CONF_BODY)
            self.assertEqual(ofono.write_wireplumber_config(), (config, None))

    def test_iso_brands_regular_configs_and_keeps_symlinks(self):
        build = load_script('tools/build-iso.py')
        boot = self.root / 'grub'
        boot.mkdir()
        config = boot / 'grub.cfg'
        config.write_text('Arch Linux')
        original = self.root / 'original'
        original.write_text('Arch Linux')
        link = boot / 'linked.cfg'
        link.symlink_to(original)
        build.brand_boot_configs(self.root, '1.2.3')
        self.assertEqual(config.read_text(), 'Nodalix OS 1.2.3')
        self.assertEqual(original.read_text(), 'Arch Linux')
        self.assertTrue(link.is_symlink())
