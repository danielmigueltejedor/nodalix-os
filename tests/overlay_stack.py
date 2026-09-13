from __future__ import annotations


class OverlayStack:
    """Reference overlay stack. Must stay aligned with OverlayManager.qml."""

    aliases = {
        "wifi": "network",
        "notifications": "notif",
        "tray": "traymenu",
        "contextmenu": "context-menu",
    }
    keyboard_ids = ("launcher", "settings", "context-menu")
    stacked_ids = (
        "dashboard",
        "launcher",
        "settings",
        "context-menu",
        "notification-center",
    )
    hover_bar_ids = (
        "audio",
        "power",
        "powerprofile",
        "notif",
        "network",
        "bluetooth",
        "traymenu",
        "workspaces",
    )

    def __init__(self) -> None:
        self.stacks: dict[str, list[str]] = {}
        self.exiting: set[tuple[str, str]] = set()

    def _id(self, overlay_id: str) -> str:
        return self.aliases.get(overlay_id, overlay_id)

    def is_hover_bar(self, overlay_id: str) -> bool:
        return self._id(overlay_id) in self.hover_bar_ids

    def stack(self, screen: str) -> list[str]:
        return list(self.stacks.get(screen, []))

    def active(self, screen: str) -> str:
        items = self.stack(screen)
        return items[-1] if items else ""

    def is_open(self, overlay_id: str, screen: str) -> bool:
        return self._id(overlay_id) in self.stack(screen)

    def is_active(self, overlay_id: str, screen: str) -> bool:
        return self.active(screen) == self._id(overlay_id)

    def z_index(self, overlay_id: str, screen: str) -> int:
        try:
            return self.stack(screen).index(self._id(overlay_id)) + 1
        except ValueError:
            return 0

    def wants_keyboard(self, screen: str) -> bool:
        return self.active(screen) in self.keyboard_ids

    def accepts_input(self, overlay_id: str, screen: str) -> bool:
        overlay_id = self._id(overlay_id)
        return self.is_open(overlay_id, screen) and (overlay_id, screen) not in self.exiting

    def _dismiss_other_hover(self, overlay_id: str, screen: str) -> None:
        keep = self._id(overlay_id)
        for other in self.stack(screen):
            if other == keep:
                continue
            if other not in self.hover_bar_ids:
                continue
            self.close(other, screen)

    def open(self, overlay_id: str, screen: str) -> str:
        overlay_id = self._id(overlay_id)
        items = self.stack(screen)
        raised = overlay_id in items
        if raised:
            items.remove(overlay_id)
        items.append(overlay_id)
        self.stacks[screen] = items
        self.exiting.discard((overlay_id, screen))
        self._dismiss_other_hover(overlay_id, screen)
        return "raised" if raised else "opened"

    def bring_to_front(self, overlay_id: str, screen: str) -> str:
        if not self.is_open(overlay_id, screen):
            return "noop"
        return self.open(overlay_id, screen)

    def close(self, overlay_id: str, screen: str) -> str:
        overlay_id = self._id(overlay_id)
        items = self.stack(screen)
        if overlay_id not in items:
            return "noop"
        items.remove(overlay_id)
        if items:
            self.stacks[screen] = items
        else:
            self.stacks.pop(screen, None)
        self.exiting.add((overlay_id, screen))
        return "closed"

    def close_top(self, screen: str) -> str:
        overlay_id = self.active(screen)
        if not overlay_id:
            return "noop"
        return self.close(overlay_id, screen)

    def toggle(self, overlay_id: str, screen: str) -> str:
        if self.is_active(overlay_id, screen):
            return self.close(overlay_id, screen)
        return self.open(overlay_id, screen)

    def click_outside(self, screen: str) -> str:
        return self.close_top(screen)

    def click_overlay(self, overlay_id: str, screen: str) -> str:
        if not self.is_open(overlay_id, screen):
            return self.click_outside(screen)
        if self.is_active(overlay_id, screen):
            return "active"
        return self.bring_to_front(overlay_id, screen)

    def visible_ids(self, screen: str) -> list[str]:
        """Open overlays may keep rendering; visibility is not top-only."""
        return self.stack(screen)
