#!/usr/bin/env bash
# Run only inside the disposable Arch CI container, never on the host.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
pacman -Syu --noconfirm
pacman -S --needed --noconfirm base-devel git sudo meson ninja glib2-devel gobject-introspection ffmpeg \
  nodejs pnpm gnome-control-center gnome-shell gnome-session gdm adwaita-icon-theme \
  archiso python-gobject tar zstd gtk-update-icon-cache ell bluez python-dbus mobile-broadband-provider-info \
  librsvg xorg-xcursorgen asciidoc bash-completion evolution-data-server gi-docgen \
  gnome-keybindings python-docutils sassc
useradd -m builder
echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers
mkdir -p /src/dist
chown -R builder:builder /src/dist
if [[ -d /src/dist/kernel ]]; then
  (cd /src/dist/kernel && sha256sum --check SHA256SUMS)
  kernel_assets=(/src/dist/kernel/linux-nodalix-*.pkg.tar.zst)
  [[ ${#kernel_assets[@]} == 1 && -f ${kernel_assets[0]} ]]
  runuser -u builder -- env NODALIX_KERNEL_ASSET="${kernel_assets[0]}" \
    bash /src/tools/build-packages.sh /src/dist/packages
else
  su builder -c 'bash /src/tools/build-packages.sh /src/dist/packages'
fi

python tools/build-iso.py --assets dist/packages --work dist/iso-candidate --output dist/iso-output --prepare-only
