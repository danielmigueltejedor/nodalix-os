#!/usr/bin/env bash
# Run only inside the disposable Arch CI container, never on the host.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
pacman -Syu --noconfirm
pacman -S --needed --noconfirm base-devel git sudo meson ninja glib2-devel ffmpeg \
  nodejs pnpm gnome-control-center gnome-shell gnome-session gdm adwaita-icon-theme \
  python-gobject tar zstd gtk-update-icon-cache ell bluez python-dbus mobile-broadband-provider-info
useradd -m builder
echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers
mkdir -p /src/dist
chown -R builder:builder /src/dist
su builder -c 'bash /src/tools/build-packages.sh /src/dist/packages'
