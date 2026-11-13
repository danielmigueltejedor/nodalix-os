import hashlib
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PKGBUILD = (ROOT / "packaging/nodalix-brujula/PKGBUILD").read_text(encoding="utf-8")


class BrujulaPackagingTests(unittest.TestCase):
    def test_source_is_pinned_to_a_commit_with_a_real_checksum(self):
        commit = re.search(r"^_brujula_commit=([0-9a-f]{40})$", PKGBUILD, re.M)
        self.assertTrue(commit, "the source must be pinned to a full commit")
        self.assertIn("$_brujula_commit", PKGBUILD)
        digest = re.search(r"^sha256sums=\('([0-9a-f]{64})'\)$", PKGBUILD, re.M)
        self.assertTrue(digest, "sha256sums must hold a real checksum, not SKIP")

    def test_it_replaces_the_unpinned_upstream_package(self):
        self.assertIn("conflicts=('brujula')", PKGBUILD)
        self.assertIn("replaces=('brujula')", PKGBUILD)
        self.assertIn("provides=('brujula' 'webbrowser')", PKGBUILD)

    def test_it_is_the_default_browser(self):
        for kind in ("text/html", "x-scheme-handler/http", "x-scheme-handler/https"):
            self.assertIn(f"{kind}=dev.danielmiguelt.Brujula.desktop", PKGBUILD)
        self.assertIn("/etc/xdg/gnome-mimeapps.list", PKGBUILD)

    def test_the_build_script_builds_it_apart_from_the_release_set(self):
        script = (ROOT / "tools/build-packages.sh").read_text(encoding="utf-8")
        self.assertIn('makepkg_one "$brujula_dir"', script)
        # Nothing under $workdir is copied into the release set, so the aside builds live elsewhere.
        self.assertIn('aside="$root/dist/build-aside"', script)
        self.assertNotIn('"$workdir/nodalix-native-apps"', script)


if __name__ == "__main__":
    unittest.main()
