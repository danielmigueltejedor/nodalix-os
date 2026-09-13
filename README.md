# Nodalix OS

Nodalix is an Arch Linux desktop built around Hyprland and QuickShell.

This development branch replaces the historical prototype with the sources of
the installed Nodalix 0.1.1 system. The previous repository remains available in
Git history. This is a development baseline, not a Nodalix 0.2.0 release.

- `shell/`: installed QuickShell sources, including Caelestia.Blobs C++ sources.
- `updater/`: installed 0.1.0 updater backend and service definitions.
- `packaging/`: package recipes being migrated from the local bootstrap builds.
- `docs/baseline/`: installation provenance and original packaging definitions.

Official shell code belongs in `/etc/xdg/quickshell/nodalix/`. The
`nodalix-shell.service` user unit must supervise `/usr/bin/nodalix-shell`, which
selects `qs -c nodalix`. User preferences belong under XDG directories, outside
the source tree. The 0.2.0 work must finish and pass upgrade validation before
publishing the stable release or building the final ISO.

See [the baseline audit](docs/baseline/README.md) for known defects and release
gates. Do not use the imported upstream installation instructions to deploy this
development branch on an installed system.
