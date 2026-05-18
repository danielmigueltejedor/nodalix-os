# Nodalix Smart Window Management

Nodalix should make Hyprland feel smarter without forcing the user to manually manage every window.

## Ideas

- Small utility windows should open floating and centered
- Busy workspaces can push new large windows to the next free workspace
- Apps can have preferred workspaces
- Dialogs and password prompts should be treated as popups
- Media/WPE/Nodalix tools should behave like proper floating app panels

## Future daemon

The nodalix-window-daemon will listen to Hyprland events and apply dynamic rules beyond static windowrule config.
