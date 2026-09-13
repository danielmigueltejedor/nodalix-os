from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from overlay_stack import OverlayStack


class OverlayManagerContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.mgr = OverlayStack()

    def test_open_a_b_c(self) -> None:
        self.assertEqual(self.mgr.open("A", "DP-1"), "opened")
        self.assertEqual(self.mgr.stack("DP-1"), ["A"])
        self.assertEqual(self.mgr.open("B", "DP-1"), "opened")
        self.assertEqual(self.mgr.stack("DP-1"), ["A", "B"])
        self.assertEqual(self.mgr.open("C", "DP-1"), "opened")
        self.assertEqual(self.mgr.stack("DP-1"), ["A", "B", "C"])
        self.assertEqual(self.mgr.visible_ids("DP-1"), ["A", "B", "C"])
        self.assertTrue(self.mgr.is_open("A", "DP-1"))
        self.assertTrue(self.mgr.is_open("B", "DP-1"))
        self.assertTrue(self.mgr.is_active("C", "DP-1"))
        self.assertFalse(self.mgr.is_active("A", "DP-1"))

    def test_bring_to_front_moves_to_end(self) -> None:
        self.mgr.open("A", "DP-1")
        self.mgr.open("B", "DP-1")
        self.mgr.open("C", "DP-1")
        self.assertEqual(self.mgr.bring_to_front("A", "DP-1"), "raised")
        self.assertEqual(self.mgr.stack("DP-1"), ["B", "C", "A"])
        self.assertEqual(self.mgr.active("DP-1"), "A")

    def test_close_top_only(self) -> None:
        self.mgr.open("A", "DP-1")
        self.mgr.open("B", "DP-1")
        self.mgr.open("C", "DP-1")
        self.assertEqual(self.mgr.close_top("DP-1"), "closed")
        self.assertEqual(self.mgr.stack("DP-1"), ["A", "B"])
        self.assertTrue(self.mgr.is_open("A", "DP-1"))
        self.assertTrue(self.mgr.is_open("B", "DP-1"))

    def test_toggle_top_closes_non_top_raises(self) -> None:
        self.mgr.open("A", "DP-1")
        self.mgr.open("B", "DP-1")
        self.assertEqual(self.mgr.toggle("B", "DP-1"), "closed")
        self.assertEqual(self.mgr.stack("DP-1"), ["A"])
        self.mgr.open("B", "DP-1")
        self.assertEqual(self.mgr.toggle("A", "DP-1"), "raised")
        self.assertEqual(self.mgr.stack("DP-1"), ["B", "A"])

    def test_launcher_toggle_semantics(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.assertEqual(self.mgr.toggle("launcher", "DP-1"), "opened")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "launcher"])
        self.assertEqual(self.mgr.toggle("launcher", "DP-1"), "closed")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard"])
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("settings", "DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "launcher", "settings"])
        self.assertEqual(self.mgr.toggle("launcher", "DP-1"), "raised")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "settings", "launcher"])
        self.assertTrue(self.mgr.is_open("settings", "DP-1"))

    def test_z_order_follows_stack(self) -> None:
        self.mgr.open("A", "DP-1")
        self.mgr.open("B", "DP-1")
        self.mgr.open("C", "DP-1")
        self.assertLess(self.mgr.z_index("A", "DP-1"), self.mgr.z_index("B", "DP-1"))
        self.assertLess(self.mgr.z_index("B", "DP-1"), self.mgr.z_index("C", "DP-1"))
        self.mgr.bring_to_front("A", "DP-1")
        self.assertLess(self.mgr.z_index("B", "DP-1"), self.mgr.z_index("C", "DP-1"))
        self.assertLess(self.mgr.z_index("C", "DP-1"), self.mgr.z_index("A", "DP-1"))

    def test_keyboard_priority_is_top_only(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.assertFalse(self.mgr.wants_keyboard("DP-1"))
        self.mgr.open("launcher", "DP-1")
        self.assertTrue(self.mgr.wants_keyboard("DP-1"))
        self.mgr.open("settings", "DP-1")
        self.assertTrue(self.mgr.wants_keyboard("DP-1"))
        self.assertTrue(self.mgr.is_open("launcher", "DP-1"))
        self.assertFalse(self.mgr.is_active("launcher", "DP-1"))
        self.mgr.close_top("DP-1")
        self.assertTrue(self.mgr.wants_keyboard("DP-1"))
        self.assertEqual(self.mgr.active("DP-1"), "launcher")

    def test_click_lower_overlay_raises_not_close_top(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.assertEqual(self.mgr.click_overlay("dashboard", "DP-1"), "raised")
        self.assertEqual(self.mgr.stack("DP-1"), ["launcher", "dashboard"])
        self.assertTrue(self.mgr.is_open("launcher", "DP-1"))

    def test_click_outside_and_escape_close_top_only(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("settings", "DP-1")
        self.assertEqual(self.mgr.click_outside("DP-1"), "closed")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "launcher"])
        self.assertEqual(self.mgr.close_top("DP-1"), "closed")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard"])

    def test_monitors_are_isolated(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("settings", "HDMI-A-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "launcher"])
        self.assertEqual(self.mgr.stack("HDMI-A-1"), ["settings"])
        self.mgr.close_top("DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard"])
        self.assertEqual(self.mgr.stack("HDMI-A-1"), ["settings"])
        self.mgr.bring_to_front("dashboard", "DP-1")
        self.assertEqual(self.mgr.stack("HDMI-A-1"), ["settings"])

    def test_hover_popouts_replace_each_other(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("network", "DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "network"])
        self.mgr.open("bluetooth", "DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "bluetooth"])
        self.assertFalse(self.mgr.is_open("network", "DP-1"))
        self.assertTrue(self.mgr.is_open("dashboard", "DP-1"))

    def test_opening_stacked_dismisses_hover_not_other_stacked(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("network", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["dashboard", "launcher"])
        self.assertFalse(self.mgr.is_open("network", "DP-1"))

    def test_open_overlays_stay_visible(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("settings", "DP-1")
        self.assertEqual(self.mgr.visible_ids("DP-1"), ["dashboard", "launcher", "settings"])
        for overlay_id in ("dashboard", "launcher", "settings"):
            self.assertTrue(self.mgr.is_open(overlay_id, "DP-1"))
            self.assertNotEqual(self.mgr.z_index(overlay_id, "DP-1"), 0)

    def test_exiting_overlay_does_not_accept_input_or_count_as_active(self) -> None:
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("settings", "DP-1")
        self.mgr.close("settings", "DP-1")
        self.assertEqual(self.mgr.active("DP-1"), "launcher")
        self.assertFalse(self.mgr.accepts_input("settings", "DP-1"))
        self.assertFalse(self.mgr.is_open("settings", "DP-1"))

    def test_aliases_wifi_and_notifications(self) -> None:
        self.mgr.open("wifi", "DP-1")
        self.assertTrue(self.mgr.is_open("network", "DP-1"))
        self.mgr.open("notifications", "DP-1")
        self.assertEqual(self.mgr.stack("DP-1"), ["notif"])


if __name__ == "__main__":
    unittest.main()
