"""Idle CPU, memory and scheduling cost of Nodalix background helpers."""
from __future__ import annotations

import importlib.util
import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


def load(name: str, relative: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


class CraftHelperIsOnlyRunWhenNeededTests(unittest.TestCase):
    def test_unit_is_skipped_unless_a_craft_application_exists(self) -> None:
        unit = read("integrations/systemd/nodalix-craft-gnome.service")
        conditions = re.findall(r"^Condition\w+=\|(.+)$", unit, re.M)
        self.assertTrue(conditions)
        # All triggering conditions form one OR group; none may be a plain AND.
        self.assertEqual([], re.findall(r"^Condition\w+=[^|\n].*$", unit, re.M))
        for place in ("/usr/share/applications/", "/var/lib/flatpak/exports/",
                      "%h/.local/share/applications/", "%h/.local/share/flatpak/exports/"):
            self.assertTrue(any(place in item for item in conditions), place)
        self.assertTrue(all("ai.storyteller." in item or "artcraft-launcher" in item for item in conditions))

    def test_path_unit_starts_the_service_after_a_later_install(self) -> None:
        path = read("integrations/systemd/nodalix-craft-gnome.path")
        self.assertIn("Unit=nodalix-craft-gnome.service", path)
        self.assertIn("PathChanged=/usr/share/applications", path)
        self.assertIn("PathChanged=%h/.local/share/flatpak/exports/share/applications", path)
        self.assertIn("PathChanged=%h/.config/autostart", path)
        package = read("packaging/nodalix-integrations/PKGBUILD")
        self.assertRegex(package, r"nodalix-craft-gnome\.service nodalix-craft-gnome\.path; do")
        self.assertRegex(package, r"default\.target\.wants.*\n(?:.*\n)*?.*nodalix-craft-gnome\.path")

    def test_periodic_reconcile_skips_proc_scan_when_nothing_changes(self) -> None:
        craft = load("craft_gnome_background", "integrations/scripts/craft_gnome.py")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            ready = root / "photocraft/preferences.json"
            ready.parent.mkdir()
            ready.write_text(json.dumps({"interface": {"systemTitleBar": True}}))
            missing = root / "vectorcraft/ui.json"
            paths = [(ready, "interface", "systemTitleBar"), (missing, "engine_prefs", "systemTitleBar")]
            with mock.patch.object(craft, "settings_paths", return_value=paths), \
                 mock.patch.object(craft, "read_state", return_value={"entries": {}}), \
                 mock.patch.object(craft, "reconcile_steam_startup", return_value=False), \
                 mock.patch.object(craft, "reconcile_desktop", return_value=False), \
                 mock.patch.object(craft, "app_busy", side_effect=AssertionError("scanned /proc")) as busy:
                self.assertFalse(craft.reconcile())
            busy.assert_not_called()

    def test_busy_editor_still_defers_a_pending_change(self) -> None:
        craft = load("craft_gnome_deferred", "integrations/scripts/craft_gnome.py")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            pending = root / "photocraft/preferences.json"
            pending.parent.mkdir()
            pending.write_text(json.dumps({"interface": {}}))
            paths = [(pending, "interface", "systemTitleBar")]
            with mock.patch.object(craft, "settings_paths", return_value=paths), \
                 mock.patch.object(craft, "read_state", return_value={"entries": {}}), \
                 mock.patch.object(craft, "reconcile_steam_startup", return_value=False), \
                 mock.patch.object(craft, "reconcile_desktop", return_value=False), \
                 mock.patch.object(craft, "BASE", root / "state"):
                with mock.patch.object(craft, "app_busy", return_value=True) as busy:
                    self.assertFalse(craft.reconcile())
                busy.assert_called_once_with("photocraft")
                self.assertEqual({"interface": {}}, json.loads(pending.read_text()))
                with mock.patch.object(craft, "app_busy", return_value=False), \
                     mock.patch.object(craft, "backup_once"):
                    self.assertTrue(craft.reconcile())
                self.assertIs(json.loads(pending.read_text())["interface"]["systemTitleBar"], True)

    def test_preference_needs_update_handles_missing_and_foreign_files(self) -> None:
        craft = load("craft_gnome_needs", "integrations/scripts/craft_gnome.py")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.assertFalse(craft.preference_needs_update(root / "absent.json", "interface", "k"))
            listing = root / "list.json"
            listing.write_text("[]")
            self.assertFalse(craft.preference_needs_update(listing, "interface", "k"))
            broken = root / "broken.json"
            broken.write_text("{")
            with self.assertRaises(ValueError):
                craft.preference_needs_update(broken, "interface", "k")
            odd = root / "odd.json"
            odd.write_text(json.dumps({"interface": "text"}))
            self.assertFalse(craft.preference_needs_update(odd, "interface", "k"))


class UnattendedWorkYieldsToTheDesktopTests(unittest.TestCase):
    SERVICES = (
        "updater/nodalix-update-system.service",
        "updater/nodalix-update-nodalix.service",
        "updater/nodalix-update-firmware.service",
        "updater/nodalix-update-flatpak.service",
        "updater/nodalix-update-flatpak-user@.service",
        "updater/nodalix-update-apps.service",
    )

    def test_automatic_updates_run_at_reduced_priority(self) -> None:
        for relative in self.SERVICES:
            with self.subTest(unit=relative):
                unit = read(relative)
                self.assertIn("Nice=10\n", unit)
                self.assertIn("IOSchedulingClass=best-effort\n", unit)
                self.assertIn("IOSchedulingPriority=7\n", unit)
                # Package-lock holders must never be starved by the idle class.
                self.assertNotIn("IOSchedulingClass=idle", unit)

    def test_home_watching_layout_service_cannot_trip_the_start_limit(self) -> None:
        unit = read("integrations/systemd/nodalix-user-layout.service")
        self.assertIn("StartLimitIntervalSec=0", unit)
        self.assertIn("PathChanged=%h\n", read("integrations/systemd/nodalix-user-layout.path"))


class ZramSwapTuningTests(unittest.TestCase):
    def test_zswap_is_disabled_only_when_zram_swap_is_created(self) -> None:
        dropin = read("integrations/assets/zram-setup-tuning.conf")
        self.assertIn("[Service]", dropin)
        # A failed tuning step must never prevent swap from being created.
        for line in re.findall(r"^ExecStartPost=.*$", dropin, re.M):
            self.assertTrue(line.startswith("ExecStartPost=-"), line)
        self.assertIn("/sys/module/zswap/parameters/enabled", dropin)
        self.assertIn("vm.page-cluster=0", dropin)
        package = read("packaging/nodalix-integrations/PKGBUILD")
        self.assertIn("usr/lib/systemd/system/systemd-zram-setup@.service.d/", package)
        self.assertIn("zram-setup-tuning.conf", package)

    def test_update_scripts_do_not_break_quoting_for_paths_with_spaces(self) -> None:
        script = read("nodalix-apps/bin/nodalix-vinilo-update")
        self.assertNotIn('trap "rm -rf "$TMP"', script)
        self.assertIn("""trap 'rm -rf "$TMP" "$STAGE"' EXIT""", script)


class SteamPollerAvoidsRescanningTests(unittest.TestCase):
    def setUp(self) -> None:
        self.steam = load("steam_auto_gamemode_scan", "integrations/scripts/steam-auto-gamemode.py")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.mock_proc = mock.patch.object(self.steam, "PROC", self.root)
        self.mock_proc.start()
        self.addCleanup(self.mock_proc.stop)

    def make(self, pid: int) -> Path:
        path = self.root / str(pid)
        path.mkdir()
        return path

    def test_judged_processes_are_skipped_until_they_exec_again(self) -> None:
        self.make(10)
        self.make(11)
        (self.root / "self").mkdir()
        keys = {"10": ("1", "/bin/a"), "11": ("2", "/bin/b")}
        with mock.patch.object(self.steam, "process_key", side_effect=lambda p: keys[p.name]):
            ignored = {10: ("1", "/bin/a"), 11: ("2", "/bin/b")}
            self.assertEqual([], list(self.steam.candidate_processes({}, ignored)))
            keys["11"] = ("2", "/bin/c")  # exec() replaced the program
            self.assertEqual(["11"], [p.name for p in self.steam.candidate_processes({}, ignored)])
            self.assertNotIn(11, ignored)

    def test_tracked_and_dead_processes_are_handled(self) -> None:
        self.make(20)
        with mock.patch.object(self.steam, "process_key", return_value=("1", "/x")):
            ignored = {20: ("1", "/x"), 99: ("1", "/gone")}
            self.assertEqual([], list(self.steam.candidate_processes({20: ("1", True)}, ignored)))
            self.assertEqual({20: ("1", "/x")}, ignored)

    def test_only_processes_without_steam_app_id_get_a_final_verdict(self) -> None:
        pid = self.make(30)
        (pid / "environ").write_bytes(b"HOME=/home/u\0")
        with mock.patch.object(self.steam.os, "getuid", return_value=pid.stat().st_uid), \
             mock.patch.object(self.steam, "UID", pid.stat().st_uid):
            self.assertEqual((None, True), self.steam.judge(pid))
            (pid / "environ").write_bytes(b"SteamAppId=440\0")
            # Wine may not have mapped the game executable yet: look again later.
            self.assertEqual((None, False), self.steam.judge(pid))
            (pid / "environ").write_bytes(b"SteamAppId=0\0")
            self.assertEqual((None, True), self.steam.judge(pid))

    def test_idle_polling_is_slower_than_active_polling(self) -> None:
        self.assertGreater(self.steam.IDLE_INTERVAL, self.steam.ACTIVE_INTERVAL)


if __name__ == "__main__":
    unittest.main()
