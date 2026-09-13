from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "shell"


class StaticInvariantTests(unittest.TestCase):
    def test_quickshell_does_not_hardcode_binding_service_actions(self) -> None:
        text = (SHELL / "hypr" / "quickshell.lua").read_text(encoding="utf-8")
        self.assertIn("binds.generated.lua", text)
        self.assertNotIn("SUPER + SPACE", text)
        self.assertNotIn("SUPER+SPACE", text)
        self.assertNotRegex(text, r'hl\.bind\([^)]*launcher toggle')
        for action in ("settings toggle", "lock lock", "tools toggle", "scratchpad toggle"):
            self.assertNotIn(f"ipc call {action}", text)

    def test_binding_service_keeps_launcher_default(self) -> None:
        text = (SHELL / "services" / "BindingService.qml").read_text(encoding="utf-8")
        self.assertIn('key: "launcher"', text)
        self.assertIn('k === "launcher" ? "SUPER + SPACE"', text)
        self.assertIn("binds.generated.lua", text)

    def test_overlay_manager_open_is_not_active(self) -> None:
        text = (SHELL / "services" / "OverlayManager.qml").read_text(encoding="utf-8")
        self.assertIn("function isOpen(", text)
        self.assertIn("function isActive(", text)
        self.assertIn("function zIndex(", text)
        self.assertIn("function closeTop(", text)
        self.assertIn("function bringToFront(", text)
        self.assertRegex(text, r"return list\.length \? list\[list\.length - 1\] : \"\"")
        self.assertIn("hoverBarIds", text)
        self.assertIn("stackedIds", text)
        self.assertIn("isHoverManaged", text)
        self.assertIn('"dashboard"', text)
        self.assertIn('"launcher"', text)

    def test_stacked_overlays_stay_visible_when_open(self) -> None:
        main = (SHELL / "MainWindow.qml").read_text(encoding="utf-8")
        bar = (SHELL / "BarOverlay.qml").read_text(encoding="utf-8")
        settings = (SHELL / "panels" / "Settings.qml").read_text(encoding="utf-8")
        self.assertIn('OverlayManager.isOpen("launcher"', main)
        self.assertIn('OverlayManager.isOpen("settings"', main)
        self.assertIn("OverlayManager.isOpen(overlayId, screenName)", bar)
        self.assertNotRegex(main, r"visible:\s*OverlayManager\.isActive")
        self.assertNotRegex(bar, r"visible:\s*OverlayManager\.isActive")
        self.assertNotRegex(settings, r"visible:\s*OverlayManager\.isActive")
        self.assertIn("bringToFront", bar)
        self.assertIn("closeTop", main)

    def test_popout_hover_leave_timer(self) -> None:
        text = (SHELL / "services" / "PopoutService.qml").read_text(encoding="utf-8")
        self.assertRegex(text, r"interval:\s*600")
        self.assertIn("pinned || widgetHovered || panelHovered", text)
        self.assertIn("_coveredByStack", text)
        self.assertIn("isStacked(currentName)", text)

    def test_legacy_migration_checks_pacman_ownership(self) -> None:
        migration = (ROOT / "updater" / "migrations" / "to_0_2_0.py").read_text(encoding="utf-8")
        registry = (ROOT / "updater" / "migrations" / "registry.py").read_text(encoding="utf-8")
        self.assertIn("has_unowned_legacy_files", migration)
        self.assertIn("pacman", registry)
        self.assertIn("-Qo", registry)
        self.assertIn("/var/lib/nodalix-updater", ROOT.joinpath("updater/nodalix-updater").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
