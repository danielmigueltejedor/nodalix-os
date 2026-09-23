# Nodalix OS

Nodalix is an Arch Linux desktop built around Hyprland and QuickShell. The
`0.2.0` release provides stable desktop packages and an installable x86_64 ISO.

## Install Nodalix 0.2.0

For a fresh installation, use the ISO and follow [the installation guide](docs/iso.md).
On an up-to-date Arch installation, download and inspect the installer, then
run it as your normal user:

```fish
curl -fL https://raw.githubusercontent.com/danielmigueltejedor/nodalix-os/main/install.sh -o /tmp/nodalix-install.sh
less /tmp/nodalix-install.sh
bash /tmp/nodalix-install.sh
```

The default channel is stable. Set `NODALIX_CHANNEL=beta` to opt into preview releases. The installer obtains the release manifest from GitHub, verifies
every package checksum, installs the required Arch dependencies and performs a
single package transaction.

## Updates

Nodalix checks GitHub Releases periodically. Updates can be managed from
**Settings → Applications → Nodalix OS updates**, including Stable/Beta channel
selection, release contents, notes, restart requirements and automatic-update
controls for Nodalix, the Arch system, user applications and firmware.

The same backend is available from a terminal:

```fish
nodalix-updater check
sudo nodalix-updater update
sudo nodalix-updater channel beta
```

## Hardware profile

Fresh installations detect the processor instruction level and select the
matching official CachyOS repository. Zen 4/5 uses `cachyos-znver4`, followed by
the x86-64-v4, x86-64-v3 and generic compatibility tiers. Nodalix installs the
matching optimized kernel while preserving Arch's `linux` kernel and boot entry
as a recovery option. Set `NODALIX_SKIP_HARDWARE_PROFILE=1` only when the
installer must leave repositories and kernels untouched.

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
