# Nodalix Command Bar

Native-feeling Nodalix launcher and command palette.

This app is currently a Python + GTK3 implementation preserved from the original `local/bin/nodalix-command-bar` script and reorganized into an app folder. `src/nodalix-command-bar` is the app entrypoint and launches the Python implementation safely for the current Wayland session. The installed command remains compatible through a thin wrapper at `local/bin/nodalix-command-bar`.

## Features

- actions from `config/nodalix/actions.json` and `~/.config/nodalix/actions.json`
- desktop application discovery
- Hyprland open windows
- Hyprland workspaces
- category chips and filtering
- keyboard navigation
- launch or focus behavior for apps

## Run

```sh
apps/nodalix-command-bar/src/nodalix-command-bar
```

The existing toggle command remains:

```sh
nodalix-command-bar-toggle
```

## Install

```sh
apps/nodalix-command-bar/scripts/install-nodalix-command-bar.sh
```

## Notes

The app keeps the class/title compatibility expected by the current Hyprland rules:

- command/class: `nodalix-command-bar`
- title: `Nodalix Command Bar`

Future work can move this to Rust + GTK4/libadwaita after the behavior is fully stabilized.
