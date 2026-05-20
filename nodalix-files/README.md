# Nodalix Files

Modern file browser for Linux/Wayland with Nodalix aesthetics. Built with Tauri 2, React, TypeScript, and Tailwind CSS.

## Requirements

- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install)
- Linux: `xdg-open`, `gio` (glib2) for trash support

## Development

```bash
cd nodalix-files
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

## Features (v0.1)

- Sidebar: Home, Desktop, Downloads, Documents, Pictures, Videos, Music, `/data` (if present)
- Grid view, breadcrumbs, tabs, back/forward/up navigation
- Double-click to open folders or files (`xdg-open`)
- File-type icons (folder, image, video, document, music, archive, generic)
- New folder, rename, move to trash (`gio trash`)
