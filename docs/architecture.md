# Nodalix OS Architecture

Nodalix OS is built as a Hyprland-first desktop experience on top of CachyOS/Arch.

## Layers

- Base system: CachyOS/Arch packages and hardware support
- Desktop layer: Hyprland, Waybar, portals, notifications, lock screen and shell tools
- Nodalix layer: menu, settings, privacy center, smart window daemon and integrations
- Optional AI layer: voice, screen context and assistant actions with explicit permissions

## Principles

- Hyprland is the only official desktop target
- Defaults should feel complete from first boot
- User configuration must not be overwritten without backup
- Privacy-sensitive activity must be visible
- AI features are optional and permission-based
