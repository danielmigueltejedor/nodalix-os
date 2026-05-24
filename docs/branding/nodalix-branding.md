# Nodalix Branding Assets

This repository includes the native Nodalix branding assets used by the local system configuration.

## Native Nodalix glyph

The native Nodalix icon is generated as a custom glyph inside a patched Nerd Font.

- Glyph codepoint: `U+E00B`
- Display character: ``
- Font family: `JetBrainsMono Nerd Font Nodalix`
- Generator script: `assets/fonts/nodalix-glyph/add-nodalix-glyph.py`
- Source SVG: `assets/fonts/nodalix-glyph/nodalix-icon.svg`

The glyph is intended to replace old Omarchy/CachyOS placeholder icons in Nodalix UI components such as Waybar.

## Terminal / Fastfetch logo

The terminal welcome logo is stored at:

`config/nodalix/branding/about.txt`

This file is used by Fastfetch as the Nodalix terminal branding logo.

## Plymouth boot branding

The Plymouth theme assets are stored at:

`assets/plymouth/nodalix/`

The theme is based on the CachyOS Plymouth two-step theme, but replaces the CachyOS watermark with Nodalix branding.

Important files:

- `assets/plymouth/nodalix/nodalix.plymouth`
- `assets/plymouth/nodalix/watermark.png`
- `assets/plymouth/nodalix/nodalix-logo-source.png`

Manual install:

1. Copy the theme:

    sudo cp -a assets/plymouth/nodalix /usr/share/plymouth/themes/nodalix

2. Enable it:

    sudo plymouth-set-default-theme nodalix

3. Rebuild initramfs:

    sudo mkinitcpio -P

If Limine is used, accept the prompt to update Limine entries or run the equivalent Limine update command.
