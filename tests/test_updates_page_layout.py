"""The Updates page must not repeat the Nodalix version that About already shows."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
PAGES = (ROOT/'gnome/control-center/cc-nodalix-pages.c').read_text()


class UpdatesPageLayoutTests(unittest.TestCase):
    def test_updates_page_has_no_installed_version_label(self):
        self.assertNotIn('identity', PAGES)
        self.assertNotIn('Consultando la versión instalada', PAGES)

    def test_about_page_still_shows_the_nodalix_release(self):
        self.assertIn('cc-nodalix-release.h', (ROOT/'gnome/control-center/apply.py').read_text())


if __name__ == '__main__':
    unittest.main()
