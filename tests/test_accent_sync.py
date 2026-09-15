from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "nodalix_accent_sync", ROOT / "shell/scripts/nodalix-sync-app-accent.py"
)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class AccentSyncTests(unittest.TestCase):
    def test_fluenty_material_palette_covers_all_accent_shades(self) -> None:
        colors = {
            "primary": "#d1bcfd",
            "onPrimary": "#37265c",
            "primaryContainer": "#4e3d75",
            "onPrimaryContainer": "#eaddff",
            "secondary": "#ccc2dc",
            "onSecondary": "#332d41",
            "surfaceContainerHigh": "#2b292f",
            "onSurface": "#e7e0e8",
            "onSurfaceVariant": "#cbc4cf",
            "outline": "#948f99",
        }
        css = MODULE.fluenty_css(colors)
        for shade in (
            "Accent", "Dark1", "Dark2", "Light1", "Light2", "Light3"
        ):
            self.assertIn(f"--SystemAccentColor{shade}:", css)
        self.assertIn("--settings-sidebar-hover-bg: var(--nodalix-hover-surface) !important", css)
        self.assertIn(".contextMenuItem:hover", css)
        self.assertIn(".activeTab", css)
        self.assertIn(".btn_grey_black", css)
        self.assertIn("._20QAC4WMXm8qFE8waUT5oo:focus-within", css)
        self.assertIn("._1UBpAXP408Ez_L_mXhW5Q9._2-O4ZG0KrnSrzISHBKctFQ:hover", css)

    def test_managed_import_follows_theme_imports_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "libraryroot.custom.css"
            path.write_text('@import url("base.css");\n@import url("library.css");\n\n.test {}\n')
            managed = '@import url("nodalix-accent.css");'
            MODULE.ensure_import(path, managed)
            first = path.read_text()
            self.assertEqual(first.splitlines()[2], managed)
            MODULE.ensure_import(path, managed)
            self.assertEqual(path.read_text(), first)


if __name__ == "__main__":
    unittest.main()
