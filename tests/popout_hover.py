from __future__ import annotations

from overlay_stack import OverlayStack


class PopoutHover:
    """Reference hover policy. Must stay aligned with PopoutService.qml."""

    interval_ms = 600

    def __init__(self, stack: OverlayStack | None = None) -> None:
        self.stack = stack or OverlayStack()
        self.has_current = False
        self.current_name = ""
        self.screen = ""
        self.widget_hovered = False
        self.panel_hovered = False
        self.pinned = False
        self.timer_running = False
        self.closed_by_timer = False

    def _covered_by_stack(self) -> bool:
        if not self.has_current:
            return False
        if self.current_name not in OverlayStack.stacked_ids:
            return False
        return not self.stack.is_active(self.current_name, self.screen)

    def _eval_hover(self) -> None:
        if not self.has_current:
            self.timer_running = False
            return
        if self.pinned or self.widget_hovered or self.panel_hovered:
            self.timer_running = False
        elif self._covered_by_stack():
            self.timer_running = False
        else:
            self.timer_running = True

    def open(self, name: str, screen: str) -> None:
        self.stack.open(name, screen)
        self.has_current = True
        self.current_name = self.stack.aliases.get(name, name)
        self.screen = screen
        self.closed_by_timer = False
        self.timer_running = False

    def close(self) -> None:
        if self.has_current:
            self.stack.close(self.current_name, self.screen)
        self.has_current = False
        self.current_name = ""
        self.widget_hovered = False
        self.panel_hovered = False
        self.pinned = False
        self.timer_running = False

    def set_widget_hovered(self, value: bool) -> None:
        self.widget_hovered = value
        self._eval_hover()

    def set_panel_hovered(self, value: bool) -> None:
        self.panel_hovered = value
        self._eval_hover()

    def set_pinned(self, value: bool) -> None:
        self.pinned = value
        self._eval_hover()

    def open_stacked(self, name: str, screen: str | None = None) -> None:
        """Launcher/settings open without becoming the hover current."""
        target = screen or self.screen
        if not self.screen:
            self.screen = target
        self.stack.open(name, target)
        self._eval_hover()

    def close_stacked(self, name: str) -> None:
        self.stack.close(name, self.screen)
        self._eval_hover()

    def fire_timer(self, elapsed_ms: int = 600) -> None:
        if not self.timer_running or elapsed_ms < self.interval_ms:
            return
        if self.widget_hovered or self.panel_hovered or self.pinned:
            return
        if self._covered_by_stack():
            return
        self.closed_by_timer = True
        self.close()
