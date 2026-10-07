# GNOME Settings for Nodalix

Based on GNOME Control Center 51.0, commit
`6ae712c0454f2586f00821c3486150c1e8feb3e7` from
https://github.com/GNOME/gnome-control-center.

`apply.py` adds two native Adwaita navigation pages: System → Updates and
Appearance → Animated backgrounds. Selecting a static background disables the
video renderer. About uses `/usr/share/pixmaps/nodalix-logo.png`.

The pages use the session D-Bus service `com.nodalix.Settings`, which remains
running when Settings closes. Privileged operations use one root-owned helper
with a fixed allowlist and a polkit authentication dialog. No Quickshell service
or QML update panel participates. Checking Arch repositories uses `checkupdates`,
not a partial database refresh of the installed system.

Build the three native packages with `tools/build-settings-packages.py`.
Dependencies must already be installed, as with `tools/build-packages.sh`.
The helper exports pristine pinned Git revisions, includes the Nodalix additions,
and injects SHA256 checksums into the staged recipes. See the recipe for build
requirements. Optional source checkout arguments reuse previously fetched trees.
Shared keyboard shortcut files remain owned by Arch's `gnome-keybindings` package.

The animated renderer is based on Hanabi commit
`b18e0414447a7b5eb472c6ede7b32782f972c4ae` from
https://github.com/jeffshee/gnome-ext-hanabi (GPL-3.0-or-later).
`packaging/nodalix-video-wallpapers/gnome51.patch` ports its effect to
`Clutter.ShaderEffect` and `Cogl.Snippet`, constructs D-Bus proxies with `new`,
and cancels delayed startup before disable. It targets GNOME Shell 51;
compatibility must be tested again for later major Shell versions.
Migration reference: https://gjs.guide/extensions/upgrading/gnome-shell-51.html.

The existing 16 videos are recorded with size and SHA256 in
`gnome/settings/animated-collection.json`. The builder takes them from
`/usr/share/backgrounds/nodalix/Animados`, or `--collection`, generates poster
frames with ffmpeg and packages them as a separate source archive. Videos and
binary source archives are not committed to Git. Retain that archive in the
release source cache for future builds. The wallpaper service also discovers
user videos in `~/.local/share/backgrounds/nodalix/animated`.

Validation: native pages rendered against the real D-Bus implementation;
GNOME 51 headless compositor ran the patched renderer, applied a video and
removed overrides on disable, without extension errors. A separate GTK window
maximized and restored while the renderer was active. Automated tests cover
operation allowlisting, concurrent operation rejection, partial failures,
privileged authentication, and recovery of managed Settings icon overrides.
Real session playback is selectable after normal logout/login; newly installed
extensions are discovered at Shell startup. No forced session restart is used.
