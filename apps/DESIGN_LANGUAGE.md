# Nodalix App Design Language

Nodalix native apps should look and feel like one OS, not a collection of unrelated GTK demos.

## Visual identity

- Dark glass surfaces.
- Rounded cards and islands.
- Purple/accent highlights.
- Inter for general UI; JetBrainsMono Nerd Font / JetBrains Mono only for monospace and technical surfaces (see `assets/styles/nodalix-fonts.css`).
- Clear type hierarchy.
- Compact but readable spacing.
- Native Linux behavior.

## Branding assets

- Official Nodalix logos/icons must be loaded from shared SVG/PNG assets in `assets/brand`.
- Do not use Nerd Font private Unicode glyphs as the official Nodalix logo.
- Private glyphs are acceptable only for generic UI symbols where a missing glyph does not break branding.
- Apps must provide a plain text fallback such as `Nodalix OS` if the brand asset is missing.
- Installed system apps should prefer `/etc/nodalix/brand` or `/usr/share/nodalix/brand` so restricted users such as `greeter` can read the logo.

## Interaction

- Keyboard-first where practical.
- Pointer interactions should be smooth and predictable.
- Destructive actions must be explicit and confirmed.
- Empty states should explain what the user can do next.
- Missing system tools should degrade gracefully.

## Avoid

- Generic unstyled GTK demo look.
- Excessive clutter.
- Harsh white borders.
- Hidden destructive actions.
- Logging secrets, paths with credentials, or passwords.
