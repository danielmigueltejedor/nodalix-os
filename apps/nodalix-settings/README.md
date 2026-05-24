# Nodalix Settings

Native system settings app for Nodalix OS.

This is an early Rust + GTK4/libadwaita implementation focused on structure, visual direction, and safe read-only behavior. It is inspired by macOS System Settings and GNOME Settings, with a dark Nodalix glass aesthetic.

## Current Status

Implemented first-pass pages:

- Usuarios
- Apariencia
- Imagen de usuario
- Bluetooth
- Wi-Fi
- Red
- Actualizaciones
- Modo de energía
- Pantallas
- Audio
- Almacenamiento
- Sesión

The app reads basic local/system information where safe tools are available. Missing tools fail gracefully and show `No disponible` or a placeholder.

## Safety

This version does not:

- create or delete users
- change passwords
- change groups
- change network settings
- modify wallpapers
- run system updates
- modify disks
- reboot, shutdown, suspend, log out, or lock by default

Session action buttons show a development warning. Real session actions are intentionally not implemented yet. Future gated behavior can use:

```sh
NODALIX_SETTINGS_ENABLE_SESSION_ACTIONS=1
```

## Run

```sh
cd /home/dani/Projects/nodalix-os/apps/nodalix-settings
cargo run
```

## Development

```sh
cargo fmt
cargo build
cargo build --release
```

## Future Goals

- real AccountsService integration for avatars
- safe polkit-backed user and password management
- wallpaper and lockscreen preview/generation
- NetworkManager integration
- Bluetooth device pairing controls
- update history and checked update flow
- power profile switching with confirmation
- monitor layout editor
- PipeWire audio routing
- gated session actions with confirmation

