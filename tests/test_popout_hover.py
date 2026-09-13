from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from overlay_stack import OverlayStack
from popout_hover import PopoutHover


class PopoutHoverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.stack = OverlayStack()
        self.hover = PopoutHover(self.stack)

    def test_widget_hovered_does_not_close(self) -> None:
        self.hover.open("network", "DP-1")
        self.hover.set_widget_hovered(True)
        self.assertFalse(self.hover.timer_running)
        self.hover.fire_timer()
        self.assertTrue(self.stack.is_open("network", "DP-1"))

    def test_panel_hovered_does_not_close(self) -> None:
        self.hover.open("network", "DP-1")
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(True)
        self.assertFalse(self.hover.timer_running)
        self.hover.fire_timer()
        self.assertTrue(self.stack.is_open("network", "DP-1"))

    def test_leave_starts_timer_and_closes_after_600ms(self) -> None:
        self.hover.open("network", "DP-1")
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.assertTrue(self.hover.timer_running)
        self.hover.fire_timer(599)
        self.assertTrue(self.stack.is_open("network", "DP-1"))
        self.hover.fire_timer(600)
        self.assertFalse(self.stack.is_open("network", "DP-1"))
        self.assertTrue(self.hover.closed_by_timer)

    def test_pinned_does_not_autoclose(self) -> None:
        self.hover.open("notif", "DP-1")
        self.hover.set_pinned(True)
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.assertFalse(self.hover.timer_running)
        self.hover.fire_timer()
        self.assertTrue(self.stack.is_open("notif", "DP-1"))

    def test_network_to_bluetooth_replaces_hover(self) -> None:
        self.hover.open("network", "DP-1")
        self.hover.open("bluetooth", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["bluetooth"])
        self.assertEqual(self.hover.current_name, "bluetooth")
        self.assertFalse(self.stack.is_open("network", "DP-1"))

    def test_hover_does_not_hide_stacked_overlays(self) -> None:
        self.stack.open("dashboard", "DP-1")
        self.hover.open("network", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["dashboard", "network"])
        self.hover.open("bluetooth", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["dashboard", "bluetooth"])
        self.assertTrue(self.stack.is_open("dashboard", "DP-1"))

    def test_dashboard_leave_closes_after_600ms(self) -> None:
        self.hover.open("dashboard", "DP-1")
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.assertTrue(self.hover.timer_running)
        self.hover.fire_timer(600)
        self.assertFalse(self.stack.is_open("dashboard", "DP-1"))
        self.assertTrue(self.hover.closed_by_timer)

    def test_dashboard_stays_under_launcher_opened_during_wait(self) -> None:
        self.hover.open("dashboard", "DP-1")
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.assertTrue(self.hover.timer_running)
        self.hover.open_stacked("launcher")
        self.assertEqual(self.stack.stack("DP-1"), ["dashboard", "launcher"])
        self.assertFalse(self.hover.timer_running)
        self.hover.fire_timer(600)
        self.assertTrue(self.stack.is_open("dashboard", "DP-1"))
        self.assertTrue(self.stack.is_open("launcher", "DP-1"))
        self.assertEqual(self.hover.current_name, "dashboard")

    def test_dashboard_on_top_of_launcher_still_autoclose(self) -> None:
        self.hover.open_stacked("launcher", "DP-1")
        self.hover.open("dashboard", "DP-1")
        self.assertEqual(self.stack.stack("DP-1"), ["launcher", "dashboard"])
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.assertTrue(self.hover.timer_running)
        self.hover.fire_timer(600)
        self.assertFalse(self.stack.is_open("dashboard", "DP-1"))
        self.assertEqual(self.stack.stack("DP-1"), ["launcher"])

    def test_dashboard_resumes_timer_when_uncovered(self) -> None:
        self.hover.open("dashboard", "DP-1")
        self.hover.set_widget_hovered(False)
        self.hover.set_panel_hovered(False)
        self.hover.open_stacked("launcher")
        self.assertFalse(self.hover.timer_running)
        self.hover.close_stacked("launcher")
        self.assertTrue(self.hover.timer_running)
        self.hover.fire_timer(600)
        self.assertFalse(self.stack.is_open("dashboard", "DP-1"))


if __name__ == "__main__":
    unittest.main()
