import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PKGBUILD = (ROOT / "packaging/nodalix-native-apps/PKGBUILD").read_text(encoding="utf-8")


def package_function(name):
    match = re.search(rf"^package_{name}\(\) \{{\n(.*?)^\}}\n", PKGBUILD, re.M | re.S)
    assert match, f"package_{name}() is missing"
    return match.group(1)


class NativeAppsPackagingTests(unittest.TestCase):
    def test_one_source_builds_three_packages(self):
        self.assertIn("pkgbase=nodalix-native-apps", PKGBUILD)
        self.assertRegex(
            PKGBUILD, r"pkgname=\(nodalix-ajustes nodalix-monitor nodalix-archivador\)"
        )
        self.assertEqual(PKGBUILD.count("cargo build"), 1)
        self.assertIn("cargo build --frozen --release", PKGBUILD)
        self.assertIn("cargo fetch --locked", PKGBUILD)

    def test_version_follows_the_workspace(self):
        workspace = (ROOT / "apps/Cargo.toml").read_text(encoding="utf-8")
        wanted = re.search(r'^version = "(.+)"$', workspace, re.M).group(1)
        self.assertEqual(re.search(r"^pkgver=(.+)$", PKGBUILD, re.M).group(1), wanted)

    def test_each_app_replaces_the_gnome_package_and_keeps_its_name_working(self):
        cases = {
            "ajustes": ("gnome-control-center", "nodalix-control-center", "gnome-control-center", "org.gnome.Settings.desktop"),
            "monitor": ("gnome-system-monitor", "gnome-system-monitor", "gnome-system-monitor", "org.gnome.SystemMonitor.desktop"),
            "archivador": ("nautilus", "nautilus", "nautilus", None),
        }
        for app, (provided, replaced, command, desktop) in cases.items():
            body = package_function(f"nodalix-{app}")
            self.assertRegex(body, rf'provides=\("{provided}=')
            self.assertIn(f'"{provided}"', body.split("conflicts=")[1].split("\n")[0])
            self.assertIn(f'"{replaced}"', body.split("replaces=")[1].split("\n")[0])
            self.assertIn(f"_launcher {command} nodalix-", body)
            if desktop:
                self.assertIn(desktop, body)

    def test_archivador_answers_show_in_folder(self):
        body = package_function("nodalix-archivador")
        self.assertIn("org.freedesktop.FileManager1.service", body)
        self.assertIn("--gapplication-service", body)

    def test_every_package_ships_the_ndl_license(self):
        for app in ("ajustes", "monitor", "archivador"):
            self.assertIn("_licenses", package_function(f"nodalix-{app}"))
        self.assertIn("LICENSE-NDL", PKGBUILD)

    def test_build_script_builds_it_apart_from_the_release_set(self):
        script = (ROOT / "tools/build-packages.sh").read_text(encoding="utf-8")
        self.assertIn('makepkg_one "$native_dir"', script)
        self.assertIn('dist/native-apps', script)
        self.assertIn("rust", (ROOT / "tools/ci-build-gnome.sh").read_text(encoding="utf-8").split())


if __name__ == "__main__":
    unittest.main()
