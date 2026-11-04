import importlib.util
from pathlib import Path
import subprocess
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

    def test_zen_uses_the_native_search_command(self):
        browser = Mock()
        browser.get_id.return_value = 'zen.desktop'
        browser.get_commandline.return_value = '"/opt/Zen Browser/zen-bin" %u'
        with (patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.Subprocess, 'new') as spawn):
            provider.open_search('acción A&B')
        spawn.assert_called_once_with(['/opt/Zen Browser/zen-bin', '--search', 'acción A&B'],
                                      provider.Gio.SubprocessFlags.NONE)

    def test_firefox_flatpak_gets_one_search_term_instead_of_a_browser_flag(self):
        browser = Mock()
        browser.get_id.return_value = 'org.mozilla.firefox.desktop'
        command = '/usr/bin/flatpak run --file-forwarding org.mozilla.firefox'
        browser.get_commandline.return_value = command + ' @@u %u @@'
        with (patch.object(provider.Gio.AppInfo, 'get_default_for_uri_scheme', return_value=browser),
              patch.object(provider.Gio.Subprocess, 'new') as spawn):
            provider.open_search('--private-window')
        spawn.assert_called_once_with([*command.split(), '--search', ' --private-window'],
                                      provider.Gio.SubprocessFlags.NONE)

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
