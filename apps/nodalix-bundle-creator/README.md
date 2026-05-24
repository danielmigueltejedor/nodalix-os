# Nodalix Bundle Creator

Native tool for creating, validating, previewing, and exporting Nodalix app bundles, themes, configs, templates, and releases.

## Why it exists

Nodalix needs a consistent way to package app bundles, visual themes, wallpaper packs, icon packs, settings profiles, and technical templates without manually copying files into system locations.

## Current prototype

- Rust + GTK4/libadwaita skeleton.
- Sidebar sections for Overview, Manifest, Assets, Validation, Preview, and Export.
- Summary card loaded from `examples/example.nodalix-bundle.toml`.
- Validate button checks the bundled example manifest.
- Export is placeholder only.

## Safety

This prototype does not install bundles, overwrite user config, or write system files.

