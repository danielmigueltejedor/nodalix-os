import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    'nodalix_web_search', ROOT / 'gnome/session/web-search/provider.py')
provider = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provider)


class PointerAndSearchTests(unittest.TestCase):
    def test_shake_ignores_small_slow_or_unidirectional_motion(self):
        subprocess.run(['gjs', '-m', str(ROOT / 'tests/shake-pointer.js')], check=True)

    def write_profile(self, home, profile_root, metadata):
        root = home / profile_root
        profile = root / 'profile'
        profile.mkdir(parents=True)
        (root / 'profiles.ini').write_text(
            '[Install1234]\nDefault=profile\n[Profile0]\nDefault=1\nPath=old-profile\n')
        data = json.dumps({'metaData': metadata}).encode()
        # A single literal run is a valid LZ4 block.
        self.assertLess(len(data), 270)
        compressed = bytes([0xF0, len(data) - 15]) + data
        (profile / 'search.json.mozlz4').write_bytes(
            b'mozLz40\0' + len(data).to_bytes(4, 'little') + compressed)
        return profile

    def test_zen_opens_a_tab_with_the_selected_engine_and_encodes_the_query(self):
        browser = Mock()
        browser.get_id.return_value = 'zen.desktop'
        browser.get_commandline.return_value = '"/opt/Zen Browser/zen-bin" %u'
        with (tempfile.TemporaryDirectory() as directory,
              patch.object(provider.Path, 'home', return_value=Path(directory)),
              patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.Subprocess, 'new') as spawn):
            self.write_profile(Path(directory), '.zen',
                               {'appDefaultEngineId': 'ddg', 'defaultEngineId': 'google'})
            provider.open_search('-- acción A&B')
        spawn.assert_called_once_with(
            ['/opt/Zen Browser/zen-bin', '--new-tab', 'https://www.google.com/search?q=--+acci%C3%B3n+A%26B'],
            provider.Gio.SubprocessFlags.NONE)

    def test_firefox_flatpak_reads_its_own_profile(self):
        browser = Mock()
        browser.get_id.return_value = 'org.mozilla.firefox.desktop'
        command = '/usr/bin/flatpak run --file-forwarding org.mozilla.firefox'
        browser.get_commandline.return_value = command + ' @@u %u @@'
        with (tempfile.TemporaryDirectory() as directory,
              patch.object(provider.Path, 'home', return_value=Path(directory)),
              patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.Subprocess, 'new') as spawn):
            self.write_profile(Path(directory), '.mozilla/firefox', {'appDefaultEngineId': 'bing'})
            self.write_profile(Path(directory), '.var/app/org.mozilla.firefox/.mozilla/firefox',
                               {'appDefaultEngineId': 'ddg'})
            provider.open_search('acción A&B')
        spawn.assert_called_once_with(
            [*command.split(), '--new-tab', 'https://duckduckgo.com/?q=acci%C3%B3n+A%26B'],
            provider.Gio.SubprocessFlags.NONE)

    def test_new_installation_reads_the_xdg_profile(self):
        with (tempfile.TemporaryDirectory() as directory,
              patch.object(provider.Path, 'home', return_value=Path(directory)),
              patch.dict(os.environ, {'XDG_CONFIG_HOME': directory, 'MOZ_LEGACY_HOME': ''})):
            self.write_profile(Path(directory), 'zen', {'appDefaultEngineId': 'bing'})
            self.assertEqual(provider.gecko_search_url('zen.desktop', ['zen'], 'dos palabras'),
                             'https://www.bing.com/search?q=dos+palabras')

    def test_unknown_engine_still_uses_native_search_with_one_safe_argument(self):
        browser = Mock()
        browser.get_id.return_value = 'firefox.desktop'
        browser.get_commandline.return_value = '/usr/bin/firefox %u'
        with (tempfile.TemporaryDirectory() as directory,
              patch.object(provider.Path, 'home', return_value=Path(directory)),
              patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.Subprocess, 'new') as spawn):
            self.write_profile(Path(directory), '.mozilla/firefox',
                               {'appDefaultEngineId': 'google', 'defaultEngineId': 'custom'})
            provider.open_search('--private-window')
        spawn.assert_called_once_with(['/usr/bin/firefox', '--search', ' --private-window'],
                                      provider.Gio.SubprocessFlags.NONE)

    def test_corrupt_profile_and_multiple_install_defaults_fall_back(self):
        with (tempfile.TemporaryDirectory() as directory,
              patch.object(provider.Path, 'home', return_value=Path(directory))):
            profile = self.write_profile(Path(directory), '.zen', {'appDefaultEngineId': 'google'})
            (profile / 'search.json.mozlz4').write_bytes(b'mozLz40\0' + b'\xff' * 4)
            self.assertIsNone(provider.gecko_search_url('zen.desktop', ['zen'], 'example'))
            (profile.parent / 'profiles.ini').write_text(
                '[Install1]\nDefault=profile\n[Install2]\nDefault=other\n')
            self.assertIsNone(provider.gecko_profile(profile.parent))

    def test_empty_search_opens_the_browser_without_a_search_page(self):
        browser = Mock()
        with patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser):
            provider.open_search('')
        browser.launch.assert_called_once_with([], None)

    def test_unsupported_browser_uses_the_https_fallback(self):
        browser = Mock()
        browser.get_id.return_value = 'other.desktop'
        with (patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.AppInfo, 'launch_default_for_uri') as launch):
            provider.open_search('acción A&B')
        launch.assert_called_once_with('https://duckduckgo.com/?q=acci%C3%B3n+A%26B', None)

    def test_empty_and_excessively_long_queries_are_bounded(self):
        self.assertEqual(provider.query(['  ', ' ']), '')
        self.assertEqual(provider.query(['a' * 513]), 'a' * 512)
