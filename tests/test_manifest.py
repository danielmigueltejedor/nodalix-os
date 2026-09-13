from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("generate_manifest", ROOT / "tools/generate-manifest.py")
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "VERSION").write_text("0.2.0\n", encoding="utf-8")
        self.definition = {
            "schema_version": 1,
            "minimum_version": "0.1.1",
            "channel": "stable",
            "components": [
                {
                    "id": "nodalix-release",
                    "name": "Nodalix",
                    "package": "nodalix-release",
                    "version": "0.2.0",
                    "required": True,
                    "restart": "none",
                }
            ],
        }
        (self.root / "components.json").write_text(json.dumps(self.definition), encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_generates_checksum_and_size_from_real_asset(self) -> None:
        asset = self.root / "nodalix-release-0.2.0-1-any.pkg.tar.zst"
        asset.write_bytes(b"package")
        result = MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)
        component = result["components"][0]
        self.assertEqual(result["version"], "0.2.0")
        self.assertEqual(component["asset"], asset.name)
        self.assertEqual(component["size"], 7)
        self.assertRegex(component["sha256"], r"^[0-9a-f]{64}$")

    def test_maps_semver_prerelease_to_arch_pkgver(self) -> None:
        (self.root / "VERSION").write_text("0.2.0-beta.1\n", encoding="utf-8")
        self.definition["channel"] = "beta"
        self.definition["components"][0]["version"] = "0.2.0-beta.1"
        (self.root / "components.json").write_text(json.dumps(self.definition), encoding="utf-8")
        asset = self.root / "nodalix-release-0.2.0beta.1-1-any.pkg.tar.zst"
        asset.write_bytes(b"package")
        result = MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)
        self.assertEqual(result["version"], "0.2.0-beta.1")
        self.assertEqual(result["channel"], "beta")
        self.assertEqual(result["components"][0]["asset"], asset.name)

    def test_ignores_debug_packages(self) -> None:
        (self.root / "nodalix-release-0.2.0-1-any.pkg.tar.zst").write_bytes(b"real")
        (self.root / "nodalix-release-0.2.0-debug-1-any.pkg.tar.zst").write_bytes(b"dbg")
        result = MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)
        self.assertEqual(result["components"][0]["asset"], "nodalix-release-0.2.0-1-any.pkg.tar.zst")

    def test_rejects_missing_asset(self) -> None:
        with self.assertRaisesRegex(MODULE.ManifestError, "expected one package"):
            MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)

    def test_rejects_ambiguous_assets(self) -> None:
        for release in (1, 2):
            (self.root / f"nodalix-release-0.2.0-{release}-any.pkg.tar.zst").write_bytes(b"x")
        with self.assertRaisesRegex(MODULE.ManifestError, "expected one package"):
            MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)

    def test_rejects_duplicate_component(self) -> None:
        self.definition["components"].append(dict(self.definition["components"][0]))
        (self.root / "components.json").write_text(json.dumps(self.definition), encoding="utf-8")
        (self.root / "nodalix-release-0.2.0-1-any.pkg.tar.zst").write_bytes(b"x")
        with self.assertRaisesRegex(MODULE.ManifestError, "duplicate component"):
            MODULE.generate(self.root / "VERSION", self.root / "components.json", self.root)


if __name__ == "__main__":
    unittest.main()
