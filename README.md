# Nodalix OS

Nodalix is an Arch Linux desktop built around Hyprland and QuickShell. The
`0.2.0` line is currently distributed as public beta releases.

## Install the current beta

On an up-to-date Arch installation, download and inspect the installer, then
run it as your normal user:

```fish
curl -fL https://raw.githubusercontent.com/danielmigueltejedor/nodalix-os/main/install.sh -o /tmp/nodalix-install.sh
less /tmp/nodalix-install.sh
bash /tmp/nodalix-install.sh
```

Set `NODALIX_CHANNEL=stable` before running it when the stable channel is
published. The installer obtains the release manifest from GitHub, verifies
every package checksum, installs the required Arch dependencies and performs a
single package transaction.

## Updates

Nodalix checks GitHub Releases periodically. Updates can be managed from
**Settings → Applications → Nodalix OS updates**, including Stable/Beta channel
selection, release contents, notes and restart requirements.

The same backend is available from a terminal:

```fish
nodalix-updater check
sudo nodalix-updater update
sudo nodalix-updater channel beta
```

## Repository layout

- `shell/`: the packaged QuickShell interface.
- `phone-link/`: Enlace móvil, the integrated iOS provider and desktop files.
- `updater/`: release detection, verification, migrations and installation.
- `packaging/`: reproducible Arch package recipes.
- `release/`: the component definition and release-manifest schema.
- `tools/`: package and manifest build tools.
- `tests/`: updater, migration, overlay and packaging regression tests.

System-owned shell code is installed in `/etc/xdg/quickshell/nodalix/` and is
started by `nodalix-shell.service`. User settings and private pairing data stay
under the user's XDG directories and are never included in releases.
