# Nodalix Shell

This directory contains the package-managed QuickShell configuration for
Nodalix. The installed location is `/etc/xdg/quickshell/nodalix/` and the
configuration is selected explicitly with `qs -c nodalix`.

The system user unit is the sole supervisor. It runs `/usr/bin/nodalix-shell`,
restarts after crashes and is enabled globally for new users. Hyprland must not
start a second QuickShell process.

Mutable data belongs in `${XDG_STATE_HOME:-$HOME/.local/state}/nodalix`. The
launcher migrates missing files from the historical `quickshell` state directory
once, never overwrites newer Nodalix state and never deletes the old directory.

The `Caelestia.Blobs` module is built from `blobs-plugin/` during package and CI
builds and installed in the system Qt QML path. Generated libraries and absolute
build paths do not belong in this source tree.

Hyprland integration lives in `hypr/quickshell.lua`. Every IPC binding must name
the Nodalix configuration, for example:

```sh
qs -c nodalix ipc call launcher toggle
```

Nodalix Settings talks to `nodalix-updater` using JSON. Installing an update is
authorized through Polkit; the shell must never collect an administrator
password or pipe one into sudo.
