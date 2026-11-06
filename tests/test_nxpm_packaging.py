import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class NxpmPackagingTests(unittest.TestCase):
    def test_polkit_actions_authorize_installed_helpers_by_path(self):
        policy = (ROOT / "nxpm/share/os.nodalix.nxpm.policy").read_text(encoding="utf-8")
        paths = re.findall(r'exec\.path">([^<]+)<', policy)
        self.assertEqual(
            sorted(paths),
            ["/usr/lib/nodalix-nxpm/remove-package", "/usr/lib/nodalix-nxpm/upgrade-system"],
        )
        pkgbuild = (ROOT / "packaging/nodalix-nxpm/PKGBUILD").read_text(encoding="utf-8")
        for path in paths:
            name = Path(path).name
            self.assertIn(f'"$lib/{name}"', pkgbuild)
            self.assertTrue((ROOT / "nxpm/share" / name).is_file())

    def test_helpers_ignore_the_user_environment(self):
        for name in ("nxpm", "upgrade-system", "remove-package"):
            first = (ROOT / "nxpm/share" / name).read_text(encoding="utf-8").splitlines()[0]
            self.assertEqual(first, "#!/usr/bin/python -I")

    def test_engine_helper_names_match_the_installed_programs(self):
        engine = (ROOT / "nxpm/src/nxpm/engine.py").read_text(encoding="utf-8")
        self.assertIn('HELPER_DIR = "/usr/lib/nodalix-nxpm"', engine)
        for name in ("upgrade-system", "remove-package"):
            self.assertIn(f'("{name}", ', engine)

    def test_package_is_part_of_the_release_and_the_installer(self):
        components = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        row = next(item for item in components["components"] if item["package"] == "nodalix-nxpm")
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        self.assertEqual(row["version"], version)
        self.assertIn("nodalix-nxpm", (ROOT / "iso/installer/packages.txt").read_text(encoding="utf-8").split())
        self.assertIn('"nodalix-nxpm"', (ROOT / "packaging/nodalix-release/PKGBUILD").read_text(encoding="utf-8"))
        self.assertIn("nodalix-nxpm-$pkgver.tar.zst", (ROOT / "tools/build-packages.sh").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
