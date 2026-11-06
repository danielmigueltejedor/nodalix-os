import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class NodalixStorePackagingTests(unittest.TestCase):
    def test_store_is_a_pinned_required_component(self):
        pkgbuild = (ROOT / "packaging/nodalix-store/PKGBUILD").read_text(encoding="utf-8")
        self.assertRegex(pkgbuild, r"_store_commit=[0-9a-f]{40}\n")
        self.assertRegex(pkgbuild, r"sha256sums=\('[0-9a-f]{64}'\)")
        self.assertIn("'nodalix-nxpm'", pkgbuild)
        components = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        row = next(item for item in components["components"] if item["package"] == "nodalix-store")
        self.assertTrue(row["required"])
        self.assertEqual(row["version"], re.search(r"^pkgver=(.+)$", pkgbuild, re.M).group(1))
        self.assertIn("nodalix-store", (ROOT / "iso/installer/packages.txt").read_text(encoding="utf-8").split())
        self.assertIn('"nodalix-store"', (ROOT / "packaging/nodalix-release/PKGBUILD").read_text(encoding="utf-8"))

    def test_store_builds_in_the_release_script(self):
        script = (ROOT / "tools/build-packages.sh").read_text(encoding="utf-8")
        self.assertIn('makepkg_one "$store_dir"', script)

    def test_store_becomes_the_default_for_software_links(self):
        pkgbuild = (ROOT / "packaging/nodalix-store/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn("os.nodalix.Store.desktop", pkgbuild)
        self.assertIn("usr/share/applications/mimeapps.list", pkgbuild)
        self.assertNotIn("etc/xdg", pkgbuild)
        for handler in ("x-scheme-handler/appstream", "application/vnd.flatpak.ref"):
            self.assertIn(handler, pkgbuild)


if __name__ == "__main__":
    unittest.main()
