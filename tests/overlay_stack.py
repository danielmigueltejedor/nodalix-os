from __future__ import annotations


class OverlayStack:
    """Reference overlay stack. Must stay aligned with OverlayManager.qml."""

    aliases = {
        "wifi": "network",
        "notifications": "notif",
        "tray": "traymenu",
        "contextmenu": "context-menu",
    }

    def __init__(self) -> None:
        self.stacks: dict[str, list[str]] = {}

    def _id(self, overlay_id: str) -> str:
        return self.aliases.get(overlay_id, overlay_id)

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

    def open(self, overlay_id: str, screen: str) -> str:
        overlay_id = self._id(overlay_id)
        items = self.stack(screen)
        raised = overlay_id in items
        if raised:
            items.remove(overlay_id)
        items.append(overlay_id)
        self.stacks[screen] = items
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
