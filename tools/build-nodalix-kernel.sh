#!/usr/bin/env bash
# Build the Nodalix kernel in a separate workspace without installing it.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd -P)
work=$(realpath -m -- "${1:-$root/dist/linux-nodalix}")
mkdir -p -- "$work"
install -m644 "$root/packaging/linux-nodalix/PKGBUILD" "$work/PKGBUILD"
install -m644 "$root/packaging/nodalix-xpadneo-dkms/preserve-ble-identity.patch" "$work/"
cd -- "$work"
makepkg --noconfirm "${@:2}"
