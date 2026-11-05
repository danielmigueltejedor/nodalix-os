# Upstream attribution

The default Nodalix cursor theme uses the dark artwork from
[Capitaine Cursors r4](https://github.com/keeferrourke/capitaine-cursors/tree/r4)
by Keefer Rourke and contributors, based on KDE Breeze and licensed under
LGPL-3.0-or-later.

The package builds from the original SVGs, hotspots and animation timings.
The scalable format is used by GNOME Shell 51 and the Xcursor format includes
an exact 30 px rendition for applications using raster cursors. The stable
Nodalix theme identifier preserves the existing default selection on upgrades.

## Layout in Nodalix OS

The package keeps a signed payload in `/usr/share/nodalix/cursor-theme/Nodalix`.
Its installation script materializes a regular `/usr/share/icons/Nodalix`
directory, using hard links when the filesystem allows them. An unowned legacy
directory is backed up and directories owned by other packages are preserved.

Arch owns `/usr/share/icons/default/index.theme`. Nodalix supplies its own
default index through the Xcursor search path. At login, a user with Nodalix
selected receives a local named theme and a raster default in the XDG icon
directory. Byte-identical copies are adopted. Subsequent updates replace copies
only if they still match the recorded packaged files. Other per-user themes
and choices remain available.
