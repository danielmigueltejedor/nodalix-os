"""Portable local desktop helpers: migration, launch arguments and Steam data."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
from test_gnome_transition_030 import BRIDGE

ROOT = Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


STEAM = load('steam_sync', 'integrations/scripts/steam-desktop-sync.py')
MODE = load('auto_gamemode', 'integrations/scripts/steam-auto-gamemode.py')
CRAFT = load('craft_frames', 'integrations/scripts/craft_gnome.py')
MIGRATION = load('user_031', 'gnome/migration/user-031.py')


class SteamTests(unittest.TestCase):
    def test_external_library_and_incomplete_downloads(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'Steam'
            external = Path(temp) / 'external'
            (root / 'steamapps').mkdir(parents=True)
            (external / 'steamapps').mkdir(parents=True)
            (root / 'steamapps/libraryfolders.vdf').write_text(
                '"libraryfolders" { "0" { "path" "' + str(external) + '" } }')
            for appid, name, flags in [('7', 'A Game', '4'), ('8', 'Downloading', '2'),
                                       ('9', 'Proton 10', '4')]:
                (external / 'steamapps' / f'appmanifest_{appid}.acf').write_text(
                    f'"AppState" {{ "appid" "{appid}" "name" "{name}" "StateFlags" "{flags}" }}')
            self.assertEqual(STEAM.installed([root]), {'7': ('A Game', root)})

    def test_launcher_preserves_actions_custom_keys_and_original_backup(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            apps = root / 'applications'
            apps.mkdir()
            entry = apps / 'my-game.desktop'
            original = ('[Desktop Entry]\nName=Personal name\nExec=steam steam://run/7\n'
                        'StartupWMClass=my-native-game\nIcon=my-custom-icon\n'
                        '[Desktop Action Save]\nName=Save\nExec=custom-command\n')
            entry.write_text(original)
            with mock.patch.object(STEAM, 'installed', return_value={'7': ('Game', root)}), \
                 mock.patch.object(STEAM, 'cached_icon', return_value=None), \
                 mock.patch.object(STEAM, 'window_classes', return_value={'7': 'other'}):
                _, changed = STEAM.sync([root], apps, root / 'icons', root / 'state')
            self.assertEqual(changed, [])
            self.assertEqual(entry.read_text(), original)

    def test_native_game_and_proton_helper_classification(self):
        with tempfile.TemporaryDirectory() as temp:
            proc = Path(temp)
            (proc / 'environ').write_bytes(b'SteamAppId=7\0STEAM_COMPAT_INSTALL_PATH=/games/Test\0')
            (proc / 'exe').symlink_to('/games/Test/bin/game')
            self.assertEqual(MODE.game_info(proc), ('7', 'game'))
            (proc / 'exe').unlink()
            (proc / 'exe').symlink_to('/runtime/wine64')
            (proc / 'maps').write_text('000-111 r-xp 000 00:00 0 /games/Test/setup.exe\n')
            self.assertIsNone(MODE.game_info(proc))
            (proc / 'maps').write_text('000-111 r-xp 000 00:00 0 /games/Test/Game.exe\n')
            self.assertEqual(MODE.game_info(proc), ('7', 'Game.exe'))


class CraftTests(unittest.TestCase):
    def test_file_arguments_remain_arguments(self):
        entry = {'exec': 'flatpak run ai.storyteller.pdfcraft %F', 'name': 'PDFCraft'}
        args = ['/tmp/file with spaces.pdf', '/tmp/$(command).pdf']
        self.assertEqual(CRAFT.command_for(entry, args),
                         ['flatpak', 'run', 'ai.storyteller.pdfcraft', *args])

    def test_old_wrapper_adopts_system_path_without_recursive_target(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            entry = root / 'ai.storyteller.photocraft.desktop'
            key = CRAFT.desktop_key(entry, entry.stem, 'Desktop Entry')
            state = {'entries': {key: {'exec': 'photocraft %F'}}}
            entry.write_text('[Desktop Entry]\nStartupNotify=false\nExec="' +
                             str(CRAFT.LEGACY_WRAPPER) + '" "' + key + '" %F\n')
            with mock.patch.object(CRAFT, 'BASE', root / 'state'):
                self.assertTrue(CRAFT.reconcile_desktop(entry, state))
                self.assertIn('/usr/bin/craft-gnome-launch', entry.read_text())
                self.assertEqual(state['entries'][key]['exec'], 'photocraft %F')
                self.assertFalse(CRAFT.reconcile_desktop(entry, state))


class MigrationTests(unittest.TestCase):
    def test_bridge_accepts_031_and_rejects_mixed_package_versions(self):
        component = {'package': 'nodalix-updater', 'version': '0.3.1',
                     'asset': 'nodalix-updater-0.3.1-1-any.pkg.tar.zst',
                     'sha256': '0' * 64}
        manifest = {'schema_version': 1, 'version': '0.3.1', 'components': [component]}
        self.assertEqual(BRIDGE.select_bridge(manifest), component)
        component['version'] = '0.3.0'
        with self.assertRaises(ValueError):
            BRIDGE.select_bridge(manifest)

    def test_only_known_unit_prototypes_are_retired(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config = root / 'config'
            units = config / 'systemd/user'
            units.mkdir(parents=True)
            name = 'steam-desktop-sync.timer'
            prototype = (ROOT / 'integrations/systemd' / name).read_text()
            (units / name).write_text(prototype)
            wants = units / 'graphical-session.target.wants'
            wants.mkdir()
            (wants / name).symlink_to(units / name)
            custom = units / 'steam-auto-gamemode.service'
            custom.write_text('[Service]\nExecStart=/custom/command\n')
            self.assertEqual(MIGRATION.retire_local_units(config, root / 'backup'), [name])
            self.assertEqual((root / 'backup/units' / name).read_text(), prototype)
            self.assertTrue(custom.exists())
            self.assertFalse((wants / name).is_symlink())
            self.assertTrue((root / 'backup/wants/graphical-session.target.wants' / name).is_symlink())
            self.assertEqual(MIGRATION.retire_local_units(config, root / 'backup'), [name])

    def test_only_unmodified_old_extensions_are_archived(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            folder = root / 'data/gnome-shell/extensions/test@example'
            folder.mkdir(parents=True)
            source = folder / 'extension.js'
            source.write_text('old source')
            prototypes = [{'uuid': folder.name, 'files': {
                'extension.js': hashlib.sha256(source.read_bytes()).hexdigest()}}]
            source.write_text('my custom source')
            MIGRATION.retire_extension_prototypes(root / 'data', root / 'backup', prototypes)
            self.assertTrue(source.exists())
            source.write_text('old source')
            MIGRATION.retire_extension_prototypes(root / 'data', root / 'backup', prototypes)
            self.assertFalse(source.exists())
            self.assertTrue((root / 'backup/extensions/test@example/extension.js').exists())

    def test_disabled_extension_is_preserved(self):
        settings = mock.Mock()
        settings.get_strv.side_effect = lambda key: (
            [] if key == 'enabled-extensions' else ['nodalix-startup-cursor@local'])
        MIGRATION.enable_startup_cursor(settings)
        settings.set_strv.assert_not_called()

    def test_imported_source_hashes_match(self):
        rows = json.loads((ROOT / 'gnome/extensions/snapshots.json').read_text())
        for row in rows:
            for name, expected in row['files'].items():
                source = ROOT / 'gnome/extensions' / row['uuid'] / name
                self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(), expected, str(source))

    def test_startup_cursor_waits_for_a_visible_main_window(self):
        source = ROOT / 'gnome/extensions/nodalix-startup-cursor@local/extension.js'
        script = r'''
const fs = require('fs'), assert = require('assert');
let text = fs.readFileSync(process.argv[1], 'utf8')
    .replace(/^import .*;$/gm, '').replace('export default class', 'return class');
let now = 0, visible = false, completed = 0;
const Cursor = new Function('GLib', 'Meta', 'Shell', 'Extension', text)(
    {get_monotonic_time: () => now}, {WindowType: {NORMAL: 0}}, {}, class {});
const cursor = new Cursor(); cursor._seen = new Map();
cursor._notification = {get_startup_sequences: () => [{get_id: () => 'a',
    get_application_id: () => 'app.desktop', get_completed: () => false,
    complete: () => completed++}]};
cursor._windows = () => [{get_window_type: () => 0, minimized: false,
    get_compositor_private: () => ({is_mapped: () => visible})}];
cursor._check(); now = 10000000; cursor._check(); assert.equal(completed, 0);
visible = true; cursor._check(); now += 749999; cursor._check(); assert.equal(completed, 0);
now++; cursor._check(); assert.equal(completed, 1);
'''
        subprocess.run(['node', '-e', script, str(source)], check=True, capture_output=True)


if __name__ == '__main__':
    unittest.main()
