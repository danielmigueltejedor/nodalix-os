# Nodalix Native Apps

`apps/` contains the native and first-party application ecosystem for Nodalix OS.

The long-term goal is a coherent Linux-native desktop suite: system shell components, productivity tools, media tools, engineering tools, and developer tooling that share one visual language and safety model.

## Categories

- System apps: shell, settings, files, notifications, updates, capture, power, network, audio, Bluetooth, and OS onboarding.
- Productivity apps: documents, spreadsheets, presentations, notes, drawing, and local transfer.
- Engineering apps: CAD, scientific tools, plotting, unit conversion, airfoil, structures, and CFD research.
- Media apps: photo and video tools.
- Developer apps: Nodalix DevKit and Bundle Creator.

## Development conventions

- Each app should have `README.md`, `ROADMAP.md`, `SPEC.md`, and `STATUS.md`.
- Rust + GTK4/libadwaita is preferred for native system apps.
- Tauri is acceptable where already established, such as `nodalix-files`.
- Prototypes must not modify system configuration automatically.
- Destructive actions require confirmation.
- Wrappers live in `local/bin`.
- Shared OS configuration lives in `config/`.
- Build artifacts such as `target/`, `node_modules/`, `dist/`, and `build/` are not part of the source contract.

## Planned ecosystem

```text
apps/
├─ native-app-template
├─ nodalix-greeter
├─ nodalix-lock
├─ nodalix-settings
├─ nodalix-control-center
├─ nodalix-bar
├─ nodalix-dock
├─ nodalix-command-bar
├─ nodalix-files
├─ nodalix-notifications
├─ nodalix-wallpapers
├─ nodalix-welcome
├─ nodalix-updater
├─ nodalix-store
├─ nodalix-monitor
├─ nodalix-capture
├─ nodalix-audio
├─ nodalix-network
├─ nodalix-bluetooth
├─ nodalix-power
├─ nodalix-tweaks
├─ nodalix-bundle-creator
├─ nodalix-pages
├─ nodalix-cells
├─ nodalix-point
├─ nodalix-drop
├─ nodalix-photo
├─ nodalix-video
├─ nodalix-draw
├─ nodalix-notes
├─ nodalix-cad
├─ nodalix-lab
├─ nodalix-plot
├─ nodalix-units
├─ nodalix-airfoil
├─ nodalix-structures
├─ nodalix-cfd
└─ nodalix-devkit
```

## Current implementation model

Existing apps are preserved. New apps start as documented prototypes or minimal Rust/GTK skeletons. Nothing in this directory should replace Waybar, modify Hyprland autostart, edit greetd, or install services without an explicit integration step.

