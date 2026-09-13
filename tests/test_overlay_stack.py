from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from overlay_stack import OverlayStack


class OverlayManagerContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.mgr = OverlayStack()

    def test_push_remove_active_bring_to_front(self) -> None:
        self.assertEqual(self.mgr.open("A", "eDP-1"), "opened")
        self.assertEqual(self.mgr.stack("eDP-1"), ["A"])
        self.assertEqual(self.mgr.open("B", "eDP-1"), "opened")
        self.assertEqual(self.mgr.stack("eDP-1"), ["A", "B"])
        self.assertEqual(self.mgr.bring_to_front("A", "eDP-1"), "raised")
        self.assertEqual(self.mgr.stack("eDP-1"), ["B", "A"])
        self.assertEqual(self.mgr.active("eDP-1"), "A")
        self.assertEqual(self.mgr.close("A", "eDP-1"), "closed")
        self.assertEqual(self.mgr.stack("eDP-1"), ["B"])
        self.assertEqual(self.mgr.close("B", "eDP-1"), "closed")
        self.assertEqual(self.mgr.stack("eDP-1"), [])

    def test_close_top_walks_the_stack(self) -> None:
        self.mgr.open("dashboard", "DP-1")
        self.mgr.open("launcher", "DP-1")
        self.mgr.open("context-menu", "DP-1")
        self.assertEqual(self.mgr.close_top("DP-1"), "closed")
        self.assertEqual(self.mgr.active("DP-1"), "launcher")
        self.assertEqual(self.mgr.close_top("DP-1"), "closed")
        self.assertEqual(self.mgr.active("DP-1"), "dashboard")
        self.assertEqual(self.mgr.close_top("DP-1"), "closed")
        self.assertEqual(self.mgr.active("DP-1"), "")

    def test_screens_do_not_contaminate(self) -> None:
        self.mgr.open("A", "screen1")
        self.mgr.open("B", "screen1")
        self.mgr.open("C", "screen2")
        self.assertEqual(self.mgr.stack("screen1"), ["A", "B"])
        self.assertEqual(self.mgr.stack("screen2"), ["C"])
        self.assertEqual(self.mgr.active("screen1"), "B")
        self.assertEqual(self.mgr.active("screen2"), "C")


if __name__ == "__main__":
    unittest.main()
