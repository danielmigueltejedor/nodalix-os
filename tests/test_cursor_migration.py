from __future__ import annotations

import os
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MIGRATOR = ROOT / "packaging/nodalix-cursor-theme/migrate-cursor-theme"
PKGBUILD = ROOT / "packaging/nodalix-cursor-theme/PKGBUILD"
SYNC = runpy.run_path(str(PKGBUILD.with_name("sync-session-cursor.py")))["synchronize"]


class CursorMigrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)

        payload = (
            self.root
            / "usr/share/nodalix/cursor-theme/Nodalix"
        )
        (payload / "cursors").mkdir(parents=True)
        default = payload.parent / "default/index.theme"
        default.parent.mkdir()
        default.write_text("[Icon Theme]\nInherits=Nodalix,Adwaita\n")
        (payload / "index.theme").write_text(
            "[Icon Theme]\nName=Nodalix\n",
            encoding="utf-8",
        )
        (payload / "cursors/left_ptr").write_text(
            "packaged",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_migrator(self) -> subprocess.CompletedProcess[str]:
        env = dict(os.environ)
        env["NODALIX_ROOT"] = str(self.root)

        return subprocess.run(
            [str(MIGRATOR)],
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_unowned_legacy_cursor_is_backed_up_and_replaced(self) -> None:
        legacy = self.root / "usr/share/icons/Nodalix"
        (legacy / "cursors").mkdir(parents=True)
        (legacy / "index.theme").write_text(
            "legacy",
            encoding="utf-8",
        )
        (legacy / "cursors/left_ptr").write_text(
            "legacy pointer",
            encoding="utf-8",
        )

        result = self.run_migrator()

        self.assertEqual(result.returncode, 0)
        self.assertFalse(legacy.is_symlink())
        self.assertEqual((legacy / "cursors/left_ptr").read_text(), "packaged")

        state = (
            self.root
            / "var/lib/nodalix-updater/migrations/cursor-theme-layout-v1"
        )
        backups = list(state.glob("legacy-Nodalix-*"))

        self.assertEqual(len(backups), 1)
        self.assertEqual(
            (backups[0] / "cursors/left_ptr").read_text(),
            "legacy pointer",
        )
        self.assertTrue((state / "latest").is_file())

    def test_migration_is_idempotent(self) -> None:
        first = self.run_migrator()
        second = self.run_migrator()

        self.assertEqual(first.returncode, 0)
        self.assertEqual(second.returncode, 0)

        legacy = self.root / "usr/share/icons/Nodalix"
        self.assertFalse(legacy.is_symlink())
        self.assertEqual((legacy / "cursors/left_ptr").read_text(), "packaged")

    def test_foreign_symlink_is_never_replaced(self) -> None:
        legacy = self.root / "usr/share/icons/Nodalix"
        legacy.parent.mkdir(parents=True)
        legacy.symlink_to("/some/other/theme")

        result = self.run_migrator()

        self.assertEqual(result.returncode, 0)
        self.assertTrue(legacy.is_symlink())
        self.assertEqual(
            os.readlink(legacy),
            "/some/other/theme",
        )
        self.assertIn("leaving it untouched", result.stderr)

    def test_old_absolute_symlink_is_replaced_by_real_cursor_files(self):
        legacy = self.root / "usr/share/icons/Nodalix"
        legacy.parent.mkdir(parents=True)
        legacy.symlink_to("/usr/share/nodalix/cursor-theme/Nodalix")
        self.assertEqual(self.run_migrator().returncode, 0)
        self.assertFalse(legacy.is_symlink())
        self.assertEqual((legacy / "cursors/left_ptr").read_text(), "packaged")
        self.assertEqual((legacy / "cursors/left_ptr").stat().st_ino,
                         (self.root / "usr/share/nodalix/cursor-theme/Nodalix/cursors/left_ptr").stat().st_ino)

    def test_sync_adopts_identical_steam_copies_and_updates_them(self):
        import shutil
        source = self.root / "usr/share/nodalix/cursor-theme/Nodalix"
        state = self.root / "state"
        icons = self.root / "icons"
        icons.mkdir()
        for name in ("Nodalix", "default"):
            shutil.copytree(source, icons / name)
        SYNC(source, icons, state)
        for name in ("Nodalix", "default"):
            self.assertTrue((state / f"{name}.json").is_file())
        (source / "cursors/left_ptr").write_text("updated")
        (source / "index.theme").write_text("[Icon Theme]\nName=Nodalix\nInherits=Adwaita\n")
        SYNC(source, icons, state)
        for name in ("Nodalix", "default"):
            self.assertEqual((icons / name / "cursors/left_ptr").read_text(), "updated")

    def test_sync_preserves_an_explicit_custom_theme_and_edit(self):
        source = self.root / "usr/share/nodalix/cursor-theme/Nodalix"
        state = self.root / "state"
        icons = self.root / "icons"
        default = icons / "default/index.theme"
        default.parent.mkdir(parents=True)
        default.write_text("[Icon Theme]\nInherits=Custom\n")
        SYNC(source, icons, state)
        (icons / "Nodalix/cursors/left_ptr").write_text("customized")
        (source / "index.theme").write_text("[Icon Theme]\nName=Nodalix new\n")
        SYNC(source, icons, state)
        self.assertEqual(default.read_text(), "[Icon Theme]\nInherits=Custom\n")
        self.assertEqual((icons / "Nodalix/cursors/left_ptr").read_text(), "customized")

    def test_package_does_not_claim_legacy_cursor_path(self) -> None:
        pkgbuild = PKGBUILD.read_text(encoding="utf-8")

        self.assertIn(
            "$pkgdir/usr/share/nodalix/cursor-theme/Nodalix",
            pkgbuild,
        )
        self.assertNotIn(
            "$pkgdir/usr/share/icons/Nodalix",
            pkgbuild,
        )
        self.assertIn(
            "install=nodalix-cursor-theme.install",
            pkgbuild,
        )
        self.assertIn(
            "$pkgdir/usr/lib/nodalix/migrate-cursor-theme",
            pkgbuild,
        )


if __name__ == "__main__":
    unittest.main()
