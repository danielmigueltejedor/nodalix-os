# Nodalix Files

Modern file browser for Linux/Wayland (Hyprland) with Nodalix aesthetics. Tauri 2 + React + TypeScript + Tailwind.

## Requirements

- pnpm
- Rust toolchain
- Linux: `xdg-open`, `gio` (trash), optional `localsend` / Flatpak

## Development

```bash
cd apps/nodalix-files
pnpm install
pnpm tauri dev
```

## Debug performance

```bash
NODALIX_FILES_DEBUG=1 pnpm tauri dev
```

Logs timings to stderr (Rust) and browser console (React).

## Features (v0.2)

- Context menus for files/folders and sidebar (no native browser menus)
- Copy / cut / paste (in-app clipboard), move to trash via `gio trash`
- Modals: rename, properties, move to, open with, folder color/icon
- Sidebar pins persisted in `~/.config/nodalix-files/sidebar.json`
- Folder appearance in `~/.config/nodalix-files/folder-customization.json`
- Hyprland window chrome: close only (no minimize)
- Fast startup via single `startup_bundle` IPC call
- Directory listing cache + virtualized grid
- Finder-like grid (no visible cell borders)

## Visual language

Nodalix Files follows the shared Nodalix OS dark/glass typography style. The UI font stack is centered on JetBrains Mono / JetBrainsMono Nerd Font with Inter and system fallbacks for a compact, native-feeling interface.

## Build

```bash
pnpm tauri build
```
