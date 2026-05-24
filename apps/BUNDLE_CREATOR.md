# Nodalix Bundle Creator

## Vision

Nodalix Bundle Creator is the native tool for creating, validating, previewing, and exporting Nodalix app bundles, themes, configs, templates, and release packages.

## Bundle types

- `app`
- `theme`
- `wallpaper-pack`
- `icon-pack`
- `settings-profile`
- `hyprland-config`
- `waybar-config`
- `engineering-template`
- `document-template`

## Current status

The first GTK prototype exists under `apps/nodalix-bundle-creator`.

## Safety

The app must not install bundles system-wide, overwrite user config, or apply shell changes until a future explicit integration phase with backups and confirmation.

## Next steps

1. Formalize manifest schema.
2. Add local validation.
3. Add bundle preview.
4. Add export to a local artifact directory.

