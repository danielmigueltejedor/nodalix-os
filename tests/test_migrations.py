from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
sys.path.insert(0, str(ROOT / "updater"))

from overlay_stack import OverlayStack  # noqa: E402

SPEC = importlib.util.spec_from_loader(
    "nodalix_updater",
    importlib.machinery.SourceFileLoader("nodalix_updater", str(ROOT / "updater/nodalix-updater")),
)
UPDATER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(UPDATER)

from migrations.registry import remove_unowned_legacy_files, run_migrations  # noqa: E402
from migrations.to_0_2_0 import LEGACY_UNOWNED_FILES, applies_0_1_1_to_0_2_0  # noqa: E402


class OverlayStackTests(unittest.TestCase):
    def setUp(self) -> None:
        self.stack = OverlayStack()

    def test_open_a_then_b(self) -> None:
        self.stack.open("A", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["A"])
        self.stack.open("B", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["A", "B"])
        self.assertEqual(self.stack.active("DP-1"), "B")

    def test_reopen_a_moves_to_front(self) -> None:
        self.stack.open("A", "DP-1")
        self.stack.open("B", "DP-1")
        self.assertEqual(self.stack.open("A", "DP-1"), "raised")
        self.assertEqual(self.stack.stack("DP-1"), ["B", "A"])
        self.assertEqual(self.stack.active("DP-1"), "A")
        self.assertEqual(self.stack.z_index("A", "DP-1"), 2)
        self.assertEqual(self.stack.z_index("B", "DP-1"), 1)

    def test_close_top_then_lower(self) -> None:
        self.stack.open("A", "DP-1")
        self.stack.open("B", "DP-1")
        self.stack.close("A", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["B"])
        self.stack.close("B", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), [])
        self.assertEqual(self.stack.active("DP-1"), "")

    def test_monitors_are_isolated(self) -> None:
        self.stack.open("A", "DP-1")
        self.stack.open("B", "DP-1")
        self.stack.open("C", "DP-2")
        self.assertEqual(self.stack.stack("DP-1"), ["A", "B"])
        self.assertEqual(self.stack.stack("DP-2"), ["C"])
        self.stack.close_top("DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["A"])
        self.assertEqual(self.stack.stack("DP-2"), ["C"])

    def test_toggle_closes_only_when_active(self) -> None:
        self.stack.open("launcher", "DP-1")
        self.stack.open("settings", "DP-1")
        self.assertEqual(self.stack.toggle("launcher", "DP-1"), "raised")
        self.assertEqual(self.stack.active("DP-1"), "launcher")
        self.assertEqual(self.stack.toggle("launcher", "DP-1"), "closed")
        self.assertEqual(self.stack.active("DP-1"), "settings")

    def test_aliases_wifi_and_notifications(self) -> None:
        self.stack.open("wifi", "DP-1")
        self.assertTrue(self.stack.is_open("network", "DP-1"))
        self.stack.open("notifications", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["network", "notif"])


class LegacyMigrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.owned: set[str] = set()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def owned_check(self, path: Path) -> bool:
        return str(path) in self.owned

    def migrate(self, files: tuple[str, ...]) -> dict:
        return remove_unowned_legacy_files(
            files=files,
            backup_dir=self.root / "backup",
            owned_check=self.owned_check,
        )

    def test_unowned_legacy_file_is_removed_and_backed_up(self) -> None:
        target = self.root / "etc" / "systemd" / "user" / "default.target.wants" / "nodalix-shell.service"
        target.parent.mkdir(parents=True)
        target.write_text("legacy", encoding="utf-8")
        result = self.migrate((str(target),))
        self.assertFalse(target.exists())
        self.assertEqual(len(result["removed"]), 1)
        backup = Path(result["removed"][0]["backup"])
        self.assertEqual(backup.read_text(encoding="utf-8"), "legacy")
        manifest = json.loads((self.root / "backup" / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["id"], "0.1.1-to-0.2.0")
        log = (self.root / "backup" / "migration.log").read_text(encoding="utf-8")
        self.assertIn("removed", log)

    def test_missing_legacy_file_is_a_noop(self) -> None:
        missing = self.root / "missing-file"
        result = self.migrate((str(missing),))
        self.assertEqual(result["skipped"][0]["reason"], "missing")
        self.assertEqual(result["removed"], [])

    def test_packaged_file_is_not_removed(self) -> None:
        target = self.root / "owned"
        target.write_text("keep", encoding="utf-8")
        self.owned.add(str(target))
        result = self.migrate((str(target),))
        self.assertTrue(target.exists())
        self.assertEqual(result["skipped"][0]["reason"], "owned")
        self.assertEqual(result["removed"], [])

    def test_symlink_backup_preserves_target(self) -> None:
        real = self.root / "real-unit"
        real.write_text("unit", encoding="utf-8")
        link = self.root / "wants" / "nodalix-update-check.timer"
        link.parent.mkdir()
        os.symlink(real, link)
        result = self.migrate((str(link),))
        self.assertFalse(link.exists())
        backup = Path(result["removed"][0]["backup"])
        self.assertTrue(backup.is_symlink())
        self.assertEqual(Path(os.readlink(backup)), real)
        self.assertEqual(result["removed"][0]["target"], str(real))

    def test_migration_is_idempotent(self) -> None:
        target = self.root / "legacy"
        target.write_text("once", encoding="utf-8")
        first = self.migrate((str(target),))
        second = self.migrate((str(target),))
        self.assertEqual(len(first["removed"]), 1)
        self.assertEqual(second["skipped"][0]["reason"], "missing")
        self.assertFalse(target.exists())

    def test_gate_skips_0_2_systems(self) -> None:
        self.assertTrue(applies_0_1_1_to_0_2_0("0.1.1", "0.2.0-beta.2", UPDATER.compare_versions))
        self.assertTrue(applies_0_1_1_to_0_2_0("0.1.1", "0.2.0-beta.1", UPDATER.compare_versions))
        self.assertFalse(applies_0_1_1_to_0_2_0("0.2.0-beta.1", "0.2.0-beta.2", UPDATER.compare_versions))
        self.assertFalse(applies_0_1_1_to_0_2_0("0.2.0-beta.2", "0.2.0", UPDATER.compare_versions))

    def test_run_migrations_skips_beta_to_beta(self) -> None:
        target = self.root / "etc" / "nodalix-release"
        target.parent.mkdir(parents=True)
        target.write_text("legacy", encoding="utf-8")
        results = run_migrations(
            "0.2.0-beta.1",
            "0.2.0-beta.2",
            compare_versions=UPDATER.compare_versions,
            state_dir=self.root / "state",
            owned_check=lambda path: False,
        )
        self.assertEqual(results, [])
        self.assertTrue(target.exists())

    def test_run_migrations_from_0_1_1(self) -> None:
        results = run_migrations(
            "0.1.1",
            "0.2.0-beta.2",
            compare_versions=UPDATER.compare_versions,
            state_dir=self.root / "state",
            owned_check=lambda path: False,
        )
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["id"], "0.1.1-to-0.2.0")

    def test_known_legacy_list_matches_upgrade_conflicts(self) -> None:
        expected = {
            "/etc/nodalix-release",
            "/etc/systemd/user/default.target.wants/nodalix-shell.service",
            "/etc/systemd/system/timers.target.wants/nodalix-update-check.timer",
            "/usr/lib/qt6/qml/Caelestia/Blobs/caelestia-blobs.qmltypes",
            "/usr/lib/qt6/qml/Caelestia/Blobs/libcaelestia-blobs.so",
            "/usr/lib/qt6/qml/Caelestia/Blobs/libcaelestia-blobsplugin.so",
            "/usr/lib/qt6/qml/Caelestia/Blobs/qmldir",
        }
        self.assertEqual(set(LEGACY_UNOWNED_FILES), expected)
        self.assertNotIn("/usr/lib/qt6/qml/Caelestia/Blobs/", LEGACY_UNOWNED_FILES)


class HistoryAndStatusTests(unittest.TestCase):
    def test_empty_history_message(self) -> None:
        self.assertEqual(UPDATER.format_history([]), "No hay actualizaciones registradas.")

    def test_empty_history_json_shape(self) -> None:
        payload = UPDATER.public_info({"history": []})
        self.assertEqual(payload, {"history": []})

    def test_status_has_stage_and_timestamp(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            original = UPDATER.STATUS_PATH
            UPDATER.STATUS_PATH = Path(raw) / "status.json"
            try:
                status = UPDATER.set_status(
                    "migrating",
                    stage="migrating",
                    current_version="0.1.1",
                    target_version="0.2.0-beta.2",
                    last_error=None,
                )
            finally:
                UPDATER.STATUS_PATH = original
        self.assertEqual(status["stage"], "migrating")
        self.assertEqual(status["current_version"], "0.1.1")
        self.assertEqual(status["target_version"], "0.2.0-beta.2")
        self.assertIn("timestamp", status)
        self.assertIsNone(status["last_error"])


if __name__ == "__main__":
    unittest.main()
