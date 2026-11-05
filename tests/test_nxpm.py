"""NXPM binary-only client regression tests; no pacman mutations."""
import json
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]
API = runpy.run_path(str(ROOT / "nxpm/nxpm"), run_name="nxpm_tests")
G = API["operate"].__globals__


class NxpmTests(unittest.TestCase):
    def setUp(self):
        self.apps = {
            "graphite": {"id": "graphite", "name": "Graphite", "package": "nodalix-graphite"},
            "vinilo": {"id": "vinilo", "name": "Vinilo", "package": "nodalix-vinilo"},
        }

    def test_catalog_rejects_untrusted_names_and_duplicates(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "catalog.json"
            with patch.dict(G, {"CATALOG": path}):
                for package in ("../../tmp", "graphite", "nodalix-graphite;rm"):
                    path.write_text(json.dumps({"schema": 1, "repository": "nodalix",
                        "apps": [{"id": "graphite", "name": "Graphite", "package": package}]}))
                    with self.assertRaises(API["NxpmError"]):
                        API["catalog"]()
                path.write_text(json.dumps({"schema": 1, "repository": "nodalix",
                    "apps": [self.apps["graphite"], self.apps["graphite"]]}))
                with self.assertRaises(API["NxpmError"]):
                    API["catalog"]()

    def test_missing_binary_fails_before_install(self):
        with patch.dict(G, {"legacy_present": lambda name: False,
                            "installed": lambda package: False,
                            "repository_packages": lambda: set(),
                            "transaction": Mock()}):
            with self.assertRaisesRegex(API["NxpmError"], "Faltan paquetes"):
                API["operate"](self.apps, "install", "graphite")
            G["transaction"].assert_not_called()

    def test_legacy_install_never_triggers_build(self):
        with patch.dict(G, {"legacy_present": lambda name: True,
                            "installed": lambda package: False,
                            "repository_packages": Mock(),
                            "transaction": Mock()}):
            with self.assertRaisesRegex(API["NxpmError"], "instalación anterior"):
                API["operate"](self.apps, "update", "graphite")
            G["transaction"].assert_not_called()
            G["repository_packages"].assert_not_called()

    def test_all_updates_are_single_binary_transaction(self):
        command = Mock(return_value=0)
        with patch.dict(G, {"installed": lambda package: package == "nodalix-graphite",
                            "repository_packages": lambda: {"nodalix-graphite"},
                            "transaction": command}):
            self.assertEqual(API["operate"](self.apps, "update", all_apps=True), 0)
            command.assert_called_once_with(["-Syu", "--needed", "--", "nodalix-graphite"])

    def test_native_remove_keeps_dependencies(self):
        command = Mock(return_value=0)
        with patch.dict(G, {"installed": lambda package: True, "transaction": command}):
            API["operate"](self.apps, "remove", "vinilo")
            command.assert_called_once_with(["-R", "--", "nodalix-vinilo"])

    def test_source_compilers_not_packaged_as_app_commands(self):
        recipe = (ROOT / "packaging/nodalix-apps/PKGBUILD").read_text()
        for old in ("nodalix-graphite-update", "nodalix-hylki-update", "nodalix-apps-lib.sh",
                    'cp -a --no-preserve=ownership     "$srcdir/nodalix-apps/bin/."'):
            self.assertNotIn(old, recipe)
        self.assertIn("nxpm", recipe)


if __name__ == "__main__":
    unittest.main()
