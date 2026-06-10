#!/usr/bin/env python3
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

SRC_DIR = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(SRC_DIR))


def load_command_bar_module():
    module_path = SRC_DIR / "nodalix-command-bar.py"
    spec = importlib.util.spec_from_file_location("nodalix_command_bar", module_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


cb = load_command_bar_module()


class AppDiscoveryTests(unittest.TestCase):
    def write_desktop(self, content):
        tmp = tempfile.NamedTemporaryFile("w", suffix=".desktop", delete=False)
        tmp.write(content)
        tmp.close()
        self.addCleanup(lambda: Path(tmp.name).unlink(missing_ok=True))
        return Path(tmp.name)

    def test_nodisplay_hidden_desktop_file_still_indexes(self):
        path = self.write_desktop(
            """[Desktop Entry]
Type=Application
Name=Zen Browser
GenericName=Web Browser
Comment=Private browser
Exec=zen-browser %U
Icon=zen-browser
Categories=Network;WebBrowser;
Keywords=internet;www;browser;web;
NoDisplay=true
Hidden=true
TryExec=missing-zen-wrapper
"""
        )

        item = cb.parse_desktop_file(path)
        self.assertIsNotNone(item)
        self.assertEqual(item["title"], "Zen Browser")
        self.assertEqual(item["command"], "zen-browser")
        self.assertTrue(item["nodisplay"])
        self.assertTrue(item["hidden"])
        self.assertIn("zen", item["search_aliases"])
        self.assertIn("navegador", item["search_aliases"])

    def test_resolvable_duplicate_beats_unresolvable_local_override(self):
        hidden = cb.parse_desktop_file(
            self.write_desktop(
                """[Desktop Entry]
Type=Application
Name=Zen Browser
Exec=definitely-missing-zen %U
NoDisplay=true
"""
            )
        )
        native = cb.parse_desktop_file(
            self.write_desktop(
                """[Desktop Entry]
Type=Application
Name=Zen Browser
Exec=sh -lc 'true' %U
"""
            )
        )

        self.assertGreater(cb.app_priority(native), cb.app_priority(hidden))


if __name__ == "__main__":
    unittest.main()
