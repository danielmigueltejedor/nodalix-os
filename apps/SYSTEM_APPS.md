# System Apps

## Vision

System apps form the native Nodalix shell: startup, lock, settings, panels, notifications, file management, update flow, and quick device controls.

## Apps

- `nodalix-greeter`: greetd greeter.
- `nodalix-lock`: future lock surface.
- `nodalix-settings`: native settings.
- `nodalix-control-center`: quick controls panel prototype.
- `nodalix-bar`: future Waybar replacement prototype.
- `nodalix-dock`: future dock/task launcher prototype.
- `nodalix-command-bar`: launcher and command palette.
- `nodalix-files`: file manager.
- `nodalix-notifications`: notification center and history.
- `nodalix-wallpapers`: wallpapers and theme selection.
- `nodalix-welcome`: first-run onboarding.
- `nodalix-updater`: update management.
- `nodalix-store`: app/package discovery.
- `nodalix-monitor`: system monitor.
- `nodalix-capture`: screenshot/screen recording.
- `nodalix-audio`: audio routing and devices.
- `nodalix-network`: network management.
- `nodalix-bluetooth`: Bluetooth devices.
- `nodalix-power`: power profiles and battery.
- `nodalix-tweaks`: advanced desktop tweaks.

## Current status

Greeter, Settings, Files, Bar, Dock, and Control Center have implementation work. Other apps are scaffolded as planned modules.

## Priorities

1. Keep current shell stable.
2. Integrate Control Center into existing islands safely.
3. Add layer-shell behavior to Bar/Dock after prototypes are stable.
4. Move dangerous actions behind explicit confirmations.

