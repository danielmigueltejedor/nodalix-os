"""Leaving an animated wallpaper must also update the shared login picture."""
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import MagicMock, patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('login_settings', ROOT/'gnome/settings/service.py')
settings = importlib.util.module_from_spec(spec)
spec.loader.exec_module(settings)
poster = settings.sys.modules['wallpaper_poster']
JPEG = b'\xff\xd8\xff\xe0' + b'0' * 64


class StaticImageTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.folder = Path(folder.name)

    def test_plain_image_is_returned_and_symlinks_are_refused(self):
        image = self.folder/'a.jpg'; image.write_bytes(JPEG)
        link = self.folder/'link.jpg'; link.symlink_to(image)
        self.assertEqual(poster.static_image(image), image.resolve())
        self.assertIsNone(poster.static_image(link))
        self.assertIsNone(poster.static_image(self.folder/'missing.jpg'))

    def test_dynamic_wallpaper_uses_its_first_static_frame(self):
        image = self.folder/'day.jpg'; image.write_bytes(JPEG)
        xml = self.folder/'dynamic.xml'
        xml.write_text(f'<background><starttime><year>2024</year></starttime>'
                       f'<static><duration>9</duration><file>{image}</file></static></background>')
        self.assertEqual(poster.static_image(xml), image.resolve())
        xml.write_text('<background><static><file>relative.jpg</file></static></background>')
        self.assertIsNone(poster.static_image(xml))
        xml.write_text('<background><broken')
        self.assertIsNone(poster.static_image(xml))

    def test_jpeg_within_the_limit_is_sent_unchanged(self):
        image = self.folder/'a.jpg'; image.write_bytes(JPEG)
        with patch.object(poster.subprocess, 'run') as run:
            self.assertEqual(poster.login_image_for(image, cache=self.folder/'cache'), image.resolve())
        run.assert_not_called()

    def test_other_formats_are_converted_once_and_cached(self):
        image = self.folder/'a.png'; image.write_bytes(b'\x89PNG' + b'0' * 64)
        calls = []

        def convert(command, **kwargs):
            calls.append(command)
            Path(command[-1]).write_bytes(JPEG)
            return subprocess.CompletedProcess(command, 0)
        cache = self.folder/'cache'
        with patch.object(poster.subprocess, 'run', side_effect=convert):
            first = poster.login_image_for(image, cache=cache)
            second = poster.login_image_for(image, cache=cache)
        self.assertEqual(first, second)
        self.assertEqual(first.read_bytes(), JPEG)
        self.assertEqual(len(calls), 1)
        self.assertIn('file', calls[0])  # no network or pipe protocols for user files

    def test_failed_conversion_leaves_no_partial_file(self):
        image = self.folder/'a.webp'; image.write_bytes(b'RIFF' + b'0' * 64)
        cache = self.folder/'cache'
        with patch.object(poster.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1)):
            with self.assertRaises(ValueError):
                poster.login_image_for(image, cache=cache)
        self.assertEqual(list(cache.iterdir()), [])


class LoginPublicationTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.folder = Path(folder.name)
        environment = patch.dict(settings.os.environ, {'XDG_STATE_HOME': str(self.folder/'state'),
                                                       'XDG_CACHE_HOME': str(self.folder/'cache')})
        environment.start(); self.addCleanup(environment.stop)
        self.image = self.folder/'static.jpg'; self.image.write_bytes(JPEG)
        self.service = settings.SettingsService()

    def test_new_static_picture_is_published_once(self):
        with patch.object(settings, 'sync_login') as sync:
            self.assertTrue(self.service.publish_login(self.image))
            self.assertFalse(self.service.publish_login(self.image))
        sync.assert_called_once_with(self.image.resolve())
        # The marker survives a service restart, so Settings reopening never asks again.
        restarted = settings.SettingsService()
        with patch.object(settings, 'sync_login') as sync:
            self.assertFalse(restarted.publish_login(self.image))
        sync.assert_not_called()

    def test_cancelled_authentication_is_not_remembered(self):
        with patch.object(settings, 'sync_login', side_effect=RuntimeError('cancelled')):
            with self.assertRaises(RuntimeError):
                self.service.publish_login(self.image)
        with patch.object(settings, 'sync_login') as sync:
            self.assertTrue(self.service.publish_login(self.image))
        sync.assert_called_once()

    def test_static_choice_after_a_video_replaces_the_login_picture(self):
        shell = MagicMock(); shell.get_strv.return_value = ['OTHER']
        with patch.object(settings, 'installed_settings', return_value=shell), \
             patch.object(self.service, 'current_static_background', return_value=self.image), \
             patch.object(settings, 'sync_login') as sync:
            self.service.sync_static_login()
        sync.assert_called_once_with(self.image.resolve())

    def test_active_video_wallpaper_is_never_overwritten_by_the_poster_monitor(self):
        shell = MagicMock(); shell.get_strv.return_value = [settings.HANABI_UUID]
        with patch.object(settings, 'installed_settings', return_value=shell), \
             patch.object(self.service, 'current_static_background', return_value=self.image), \
             patch.object(settings, 'sync_login') as sync:
            self.service.sync_static_login()
        sync.assert_not_called()

    def test_failures_are_reported_without_crashing_the_service(self):
        shell = MagicMock(); shell.get_strv.return_value = []
        with patch.object(settings, 'installed_settings', return_value=shell), \
             patch.object(self.service, 'current_static_background', return_value=self.image), \
             patch.object(settings, 'sync_login', side_effect=RuntimeError('cancelled')), \
             patch('sys.stderr'):
            self.service.sync_static_login()

    def test_stopping_the_video_clears_the_lock_screen_poster_and_queues_a_sync(self):
        poster_uri = (self.folder/'cache/nodalix/wallpaper-posters/x.jpg').as_uri()
        screensaver = MagicMock()
        screensaver.get_string.return_value = poster_uri
        screensaver.props.settings_schema.list_keys.return_value = ['picture-uri']
        service = settings.SettingsService(); service.connection = MagicMock()
        with patch.object(settings, 'request_wallpaper'), patch.object(service, 'extension'), \
             patch.object(settings, 'installed_settings', return_value=screensaver), \
             patch.object(settings.GLib, 'idle_add') as idle:
            service.stop_wallpaper()
        screensaver.reset.assert_called_once_with('picture-uri')
        idle.assert_called_once_with(service.schedule_login_sync)

    def test_a_personal_screensaver_picture_is_left_alone(self):
        screensaver = MagicMock()
        screensaver.get_string.return_value = 'file:///home/me/Pictures/mine.jpg'
        screensaver.props.settings_schema.list_keys.return_value = ['picture-uri']
        with patch.object(settings, 'installed_settings', return_value=screensaver):
            self.service.reset_screensaver_still()
        screensaver.reset.assert_not_called()


if __name__ == '__main__':
    unittest.main()
