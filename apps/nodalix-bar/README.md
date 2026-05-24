# Nodalix Bar

Native GTK4/libadwaita prototype for the future Nodalix OS top bar.

This is a development preview. It does not replace Waybar, does not reserve screen space, and does not modify Hyprland config.

## Run

```bash
cargo run
```

Or from the repository root:

```bash
local/bin/nodalix-bar
```

## Prototype status

- Top glass bar mock with left, center, and right islands.
- Reads Hyprland active workspace/window when `hyprctl` exists.
- Reads network/audio/Bluetooth using common CLI tools when available.
- Opens `nodalix-control-center` from the logo/control button if the wrapper or binary is available.

Future work should add layer-shell support, real tray support, workspace interaction, and safe integration into existing Nodalix islands before replacing Waybar.

