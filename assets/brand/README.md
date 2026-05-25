# Nodalix Brand Assets

Official Nodalix logos live here as real SVG/PNG assets.

Use these files for app branding:

- `nodalix-logo-symbol.svg`: standalone Nodalix symbol.
- `nodalix-logo.svg`: symbol plus wordmark.

Do not use Nerd Font private Unicode glyphs for the official Nodalix logo in native apps. Private glyphs can render incorrectly in restricted sessions such as greetd, where the patched font may not be available or may map the codepoint differently.

Installed system paths should prefer:

1. `/etc/nodalix/brand/nodalix-logo-symbol.svg`
2. `/usr/share/nodalix/brand/nodalix-logo-symbol.svg`
3. Repository fallback during development: `/home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg`
