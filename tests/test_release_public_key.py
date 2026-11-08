import base64
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class ReleasePublicKeyTests(unittest.TestCase):
    def test_updater_ships_a_minisign_public_key(self) -> None:
        lines = (
            (ROOT / "updater" / "release.pub")
            .read_text(encoding="utf-8")
            .splitlines()
        )

        self.assertEqual(len(lines), 2)
        self.assertTrue(lines[0].startswith("untrusted comment:"))

        key = base64.b64decode(lines[1], validate=True)

        # Signature algorithm "Ed", 8-byte key id, 32-byte Ed25519 key.
        self.assertEqual(len(key), 42)
        self.assertEqual(key[:2], b"Ed")

    def test_updater_package_installs_the_public_key(self) -> None:
        pkgbuild = (
            ROOT / "packaging" / "nodalix-updater" / "PKGBUILD"
        ).read_text(encoding="utf-8")

        self.assertIn("/usr/share/nodalix/keys/release.pub", pkgbuild)


if __name__ == "__main__":
    unittest.main()
