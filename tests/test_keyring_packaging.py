import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FINGERPRINT = "3DE029728CDE1828E18B437930D356AC0F6AB1CF"


class NodalixKeyringPackagingTests(unittest.TestCase):
    def test_keyring_trusts_exactly_the_repository_key(self):
        pkgbuild = (ROOT / "packaging/nodalix-keyring/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn(f"fingerprint={FINGERPRINT}", pkgbuild)
        install = (ROOT / "packaging/nodalix-keyring/nodalix-keyring.install").read_text(encoding="utf-8")
        self.assertIn("pacman-key --populate nodalix", install)
        self.assertIn(FINGERPRINT, install)
        key = (ROOT / "packaging/nodalix-keyring/nodalix.pub.asc").read_text(encoding="utf-8")
        self.assertTrue(key.startswith("-----BEGIN PGP PUBLIC KEY BLOCK-----"))
        self.assertNotIn("PRIVATE KEY", key)

    def test_keyring_is_a_required_release_component(self):
        pkgbuild = (ROOT / "packaging/nodalix-keyring/PKGBUILD").read_text(encoding="utf-8")
        version = re.search(r"^pkgver=(.+)$", pkgbuild, re.M).group(1)
        components = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        row = next(item for item in components["components"] if item["package"] == "nodalix-keyring")
        self.assertTrue(row["required"])
        self.assertEqual(row["version"], version)
        self.assertIn("nodalix-keyring", (ROOT / "iso/installer/packages.txt").read_text(encoding="utf-8").split())
        self.assertIn('"nodalix-keyring"', (ROOT / "packaging/nodalix-release/PKGBUILD").read_text(encoding="utf-8"))
        self.assertIn('makepkg_one "$keyring_dir"', (ROOT / "tools/build-packages.sh").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
