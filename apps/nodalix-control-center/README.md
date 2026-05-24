# Nodalix Control Center

Native GTK4/libadwaita prototype for the future Nodalix OS quick settings panel.

This is a safe development preview. It does not replace Waybar, does not change Hyprland autostart, and does not require root.

## Run

```bash
cargo run
```

Or from the repository root:

```bash
local/bin/nodalix-control-center
```

## Prototype status

- Floating macOS-like quick controls panel.
- Connectivity, audio, brightness, media, focus/theme placeholders, power actions, and quick app actions.
- Dangerous power actions require confirmation.
- Missing tools such as `wpctl`, `pamixer`, `brightnessctl`, `playerctl`, `nmcli`, or `bluetoothctl` are handled gracefully.

Future integration should open this panel from Nodalix Bar's right island.

