import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    'nodalix_web_search', ROOT / 'gnome/session/web-search/provider.py')
provider = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provider)


class PointerAndSearchTests(unittest.TestCase):
    def test_shake_ignores_small_slow_or_unidirectional_motion(self):
        subprocess.run(['gjs', '-m', str(ROOT / 'tests/shake-pointer.js')], check=True)

    def test_query_launches_the_default_https_handler(self):
        with patch.object(provider.Gio.AppInfo, 'launch_default_for_uri') as launch:
            provider.open_search(provider.query(['acción', 'A&B']))
            launch.assert_called_once_with('https://duckduckgo.com/?q=acci%C3%B3n+A%26B', None)

    def test_empty_and_excessively_long_queries_are_bounded(self):
        self.assertEqual(provider.query(['  ', ' ']), '')
        self.assertEqual(provider.query(['a' * 513]), 'a' * 512)
