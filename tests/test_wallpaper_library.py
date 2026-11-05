"""Regression coverage with actual MP4 and FFmpeg output."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'gnome/settings'))
from wallpaper_library import WallpaperLibrary, inspect_video


@unittest.skipUnless(shutil.which('ffmpeg') and shutil.which('ffprobe'), 'FFmpeg tools required')
class WallpaperLibraryTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.system = self.root / 'system'
        self.system.mkdir()
        self.personal = self.root / 'personal'
        self.cache = self.root / 'cache'
        self.library = WallpaperLibrary(self.personal, (self.system,), self.cache, self.root / 'state')
        self.video = self.root / 'Vídeo breve.mp4'
        subprocess.run(['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i',
                        'testsrc2=s=64x64:r=24:d=0.75', '-c:v', 'mpeg4', str(self.video)], check=True)

    def test_import_deduplicates_bytes_and_rejects_invalid_media(self):
        duplicate = self.root / 'otro-nombre.mp4'
        shutil.copyfile(self.video, duplicate)
        broken = self.root / 'broken.mp4'
        broken.write_text('not a video')
        unsupported = self.root / 'wallpaper.jpg'
        unsupported.write_text('not a video')
        result = self.library.import_videos(map(str, (self.video, duplicate, broken, unsupported)))
        self.assertEqual((result['imported'], result['duplicates'], len(result['errors'])), (1, 1, 2))
        self.assertEqual([path.name for path in self.personal.iterdir()], [self.video.name])
        self.assertTrue(duplicate.exists())

    def test_thumbnail_is_cached_and_replaced_when_video_changes(self):
        self.library.import_videos([str(self.video)])
        self.assertEqual(list(self.library.catalog())[0]['preview'], '')
        self.assertTrue(self.library.enrich())
        item = list(self.library.catalog())[0]
        preview = Path(item['preview'])
        self.assertTrue(preview.is_file())
        self.assertEqual((item['width'], item['height'], item['fps']), (64, 64, 24))
        self.assertEqual(preview.read_bytes()[:3], b'\xff\xd8\xff')
        self.assertFalse(self.library.enrich())
        video = Path(item['path'])
        video.touch()
        self.assertTrue(self.library.enrich())
        self.assertNotEqual(list(self.library.catalog())[0]['preview'], str(preview))
        self.library.prune_cache()
        self.assertFalse(preview.exists())

    def test_cleanup_preserves_system_and_selected_items_and_removes_cache(self):
        self.library.import_videos([str(self.video)])
        selected = self.personal / self.video.name
        bundled = self.system / 'bundled.mp4'
        duplicate = self.personal / 'duplicate.mp4'
        for path in (bundled, duplicate):
            shutil.copyfile(self.video, path)
        self.library.enrich()
        self.library.favorite(str(duplicate), True)
        self.assertEqual(self.library.duplicates(str(selected)), [str(duplicate)])
        with self.assertRaises(ValueError):
            self.library.delete([str(bundled)], str(selected))
        with self.assertRaises(ValueError):
            self.library.delete([str(selected)], str(selected))
        self.library.delete([str(duplicate)], str(selected), duplicates_only=True)
        self.assertEqual({item['path'] for item in self.library.catalog()}, {str(bundled), str(selected)})
        self.assertEqual(len(list(self.cache.glob('*.jpg'))), 1)
        self.assertTrue(self.video.exists())

    def test_interpolation_retains_original_and_duration_and_can_be_cancelled(self):
        self.library.import_videos([str(self.video)])
        source = self.personal / self.video.name
        before = source.read_bytes()
        cancelled = threading.Event()
        cancelled.set()
        with self.assertRaisesRegex(ValueError, 'cancelada'):
            self.library.optimize(str(source), cancelled)
        self.assertEqual(list(self.personal.iterdir()), [source])
        self.library.optimize(str(source), threading.Event())
        output = self.personal / (source.stem + ' (60 FPS).mkv')
        details = inspect_video(output)
        self.assertEqual(details['fps'], 60)
        self.assertAlmostEqual(details['duration'], inspect_video(source)['duration'], delta=1/60)
        self.assertEqual(source.read_bytes(), before)
