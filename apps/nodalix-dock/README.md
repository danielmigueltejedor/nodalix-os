# Nodalix Dock

Native GTK4/libadwaita prototype for the future Nodalix OS dock and task launcher.

This is a safe manual preview. It does not replace existing workflow, does not reserve screen space, and does not modify Hyprland config.

## Run

```bash
cargo run
```

Or from the repository root:

```bash
local/bin/nodalix-dock
```

## Prototype status

- Glass dock with pinned Browser, Files, Terminal, Settings, and Command Bar entries.
- Reads Hyprland clients with `hyprctl clients -j` when available to show running indicators.
- Click launches the configured command.

Future work should add layer-shell placement, focus existing windows, app icon lookup, pinned app config, and drag reordering.

