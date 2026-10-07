# GNOME Settings for Nodalix

Based on GNOME Control Center 51.0, commit
`6ae712c0454f2586f00821c3486150c1e8feb3e7` from
https://github.com/GNOME/gnome-control-center.

`apply.py` adds native Adwaita navigation pages: System → Updates,
Sharing → LocalSend. Animated backgrounds are integrated directly into the
Appearance page below static backgrounds, using the same thumbnail dimensions,
rounding, selection marker and single-click interaction. Selecting a static background disables the
video renderer. About uses `/usr/share/pixmaps/nodalix-logo.png`, with a 96 pixel height clamp
and shrinking enabled for high resolution images. A 96 pixel height request
also keeps the image visible when the long About page measures its minimum size.

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

Playback status queries both the extension and the renderer. A new extension
that Shell has not discovered is queued in enabled-extensions for the next
normal login, preserving other extensions. Its poster is applied immediately;
the UI reports the pending login instead of claiming playback. Static selection
also cancels pending video activation.

The existing 16 videos are recorded with size and SHA256 in
`gnome/settings/animated-collection.json`. The builder takes them from
`/usr/share/backgrounds/nodalix/Animados`, or `--collection`, generates poster
frames with ffmpeg and packages them as a separate source archive. The 480-pixel
gallery previews and native-resolution desktop stills are separate assets. Videos and
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

LocalSend uses the existing user GLocalSend schema and the same
`com.nodalix.LocalSend` bridge that Nautilus uses. It controls activation,
discovery, alias, receive folder, login activation, auto-disable, auto-accept
and favorite fingerprints. No second receiver or separate settings store is
created. Preferences take effect through the extension's settings-change handler
without restarting a transfer. The existing TLS identity and certificates are
preserved. Nautilus includes a shortcut to Sharing → LocalSend. New bridge modules
load at the next normal login; until then the page can edit preferences and shows
a clear message that the runtime connection requires a new login.

The Sharing panel remains visible without gnome-user-share or Rygel: LocalSend
is an independent reason to expose it. Those upstream sharing rows continue
to follow their existing availability checks.

The GNOME package depends on CUPS and cups-pk-helper. The system migration
enables cups.socket, and installation includes both packages so the native
Printers panel has a printing service. See https://openprinting.github.io/cups/doc/admin.html.
GNOME 51 does not discover a newly installed local extension in the active
session, and ReloadExtension is unsupported; see
https://github.com/GNOME/gnome-shell/blob/51.0/js/ui/shellDBus.js.

GDM uses `/usr/share/pixmaps/nodalix-login-logo.svg`, a separate 64×64 viewport
with the unchanged PNG artwork embedded. GNOME 51 loads the logo at its intrinsic
size (`load_file_async` without requested dimensions), so the large About image
must not be used directly. The package generates the self-contained SVG at build
time; external image references are avoided because the texture loader may lack
a base URI. The native St loader was verified at 1× and 2× scaling: 64×64 and
128×128 respectively. No GDM service restart is used when applying the database.
Source: https://github.com/GNOME/gnome-shell/blob/51.0/js/gdm/loginDialog.js.


Lock/unlock playback continuity: the pinned renderer patch supports the `user`
and `unlock-dialog` session modes. When lock-screen playback is disabled it
pauses, retains its decoded frame/window, and reevaluates automatic playback on
unlock without overriding user pause. A real disable/logout still removes all
processes, overrides and signals. The default/additional startup delay is zero;
window discovery is polled at 100 ms and the transition takes 150 ms.
Session-mode reference: https://gjs.guide/extensions/topics/session-modes.html.
An isolated GNOME 51 test verified the same renderer PID and clone source across
both modes, paused playback while locked, playback at the first 151 ms unlock
check, manual-pause preservation and renderer shutdown on disable. Installed
4K fallback frames preserve original video hashes and do not enlarge the gallery
thumbnails. Updated extension modules/metadata require a normal new login.


The login background is a separate Nodalix extension limited to GNOME 51's
`gdm` session mode; it is packaged with Settings and enabled only in GDM's dconf
profile. It creates non-reactive native BackgroundManagers behind the greeter
controls in the lock-dialog group, with the same blur/brightness as GNOME's
unlockDialog (90 px / 0.65). Monitor and scale signals and all background managers
are removed on disable. No upstream theme resources or authentication methods
are patched. A real `--mode=gdm` isolated compositor and screenshot confirmed
visible wallpaper and compact logo with the normal user controls above them.
Configuration preserves existing GDM extension entries and the previously
selected login image across package upgrades. The greeter uses a public still;
per-user wallpaper changes are not automatically synchronized to the shared
login-screen profile. GDM 51 dynamically allocates greeter users, so public
image readability was verified with an unprivileged account rather than a
persistent `gdm` user. No live GDM restart/logout is used.
Primary references:
https://github.com/GNOME/gnome-shell/blob/51.0/js/ui/unlockDialog.js,
https://gjs.guide/extensions/overview/anatomy.html,
https://github.com/GNOME/gdm/blob/51.0/NEWS.
