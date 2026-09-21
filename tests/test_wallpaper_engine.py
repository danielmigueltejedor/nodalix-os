import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, Mock

spec = importlib.util.spec_from_file_location('wallpaper', Path(__file__).parents[1] / 'shell/scripts/nodalix-wallpaper.py')
wallpaper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wallpaper)

class WallpaperEngineTests(unittest.TestCase):
    def test_configuration_keeps_theming_in_nodalix_and_shared_rendering(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(wallpaper.os.environ, {'XDG_CONFIG_HOME': tmp}):
            wallpaper.configure(False)
            data = json.loads((Path(tmp) / 'nodalix/wallpaper/config.json').read_text())
            self.assertEqual(data['theme']['policy'], 'off')
            self.assertFalse(data['paper']['videoMultiProcess'])
            self.assertFalse(data['playback']['fullscreen'])
            self.assertEqual(data['paper']['idlePauseSeconds'], 30)

    def test_failed_apply_keeps_old_renderer(self):
        with tempfile.NamedTemporaryFile() as media, patch.object(wallpaper, 'configure'), patch.object(wallpaper, 'stop_legacy') as stop, patch.object(wallpaper.subprocess, 'run') as run:
            run.side_effect = [Mock(returncode=0), Mock(returncode=0), wallpaper.subprocess.CalledProcessError(1, 'apply')]
            with self.assertRaises(wallpaper.subprocess.CalledProcessError):
                wallpaper.apply(media.name)
            stop.assert_not_called()

    def test_missing_media_never_starts_service(self):
        with patch.object(wallpaper.subprocess, 'run') as run, self.assertRaises(FileNotFoundError):
            wallpaper.apply('/nonexistent/nodalix-wallpaper-test')
        run.assert_not_called()
