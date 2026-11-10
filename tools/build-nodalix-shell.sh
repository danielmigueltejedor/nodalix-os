#!/usr/bin/env bash
# Assemble the source package in an isolated Arch VM or clean chroot.
# This script builds release packages and never installs them on the host.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
pkgname=nodalix-gnome-shell
if [[ "${1:-}" == '--preview' ]]; then
  pkgname=nodalix-shell-preview
  shift
fi
version=$(tr -d '[:space:]' < "$root/VERSION")
pkgver=$(sed -n 's/^pkgver=//p' "$root/packaging/nodalix-gnome-shell/PKGBUILD")
if [[ "$version" != "$pkgver" ]]; then
  echo "Nodalix Shell package $pkgver does not match Nodalix OS $version" >&2
  exit 1
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$root/packaging/nodalix-gnome-shell/PKGBUILD" "$work/PKGBUILD"
if [[ "$pkgname" == nodalix-shell-preview ]]; then
  sed -i -e 's/^pkgname=.*/pkgname=nodalix-shell-preview/' \
    -e '/^pkgrel=/a epoch=1' \
    -e "s/^conflicts=.*/conflicts=('gnome-shell' 'nodalix-gnome-shell')/" "$work/PKGBUILD"
fi
module_ids=$(python3 -c 'import json, sys; print(*json.load(sys.stdin)["bundled"], sep="\n")'   < "$root/gnome/nodalix-shell/modules.json")
module_sources=()
while IFS= read -r uuid; do
  [[ "$uuid" != 'nowplaying@epogonii.github.io' ]] || continue
  if [[ ! -f "$root/gnome/extensions/$uuid/extension.js" ]]; then
    echo "Missing source for Nodalix Shell module $uuid" >&2
    exit 1
  fi
  module_sources+=("gnome/extensions/$uuid")
done <<< "$module_ids"
tar --exclude='*/__pycache__' --exclude='gschemas.compiled' --zstd -C "$root" -cf "$work/nodalix-shell-overlays.tar.zst" \
  gnome/nodalix-shell \
  tools/prepare-nodalix-shell.py \
  tools/patch-nowplaying-multimonitor.py \
  "${module_sources[@]}"
sum=$(sha256sum "$work/nodalix-shell-overlays.tar.zst" | cut -d' ' -f1)
sed -i "s/AUTO_OVERLAY_SHA256/$sum/" "$work/PKGBUILD"
if [[ "${1:-}" == '--prepare-only' ]]; then
  target="$root/dist/$pkgname-source"
  mkdir -p "$target"
  cp "$work/PKGBUILD" "$work/nodalix-shell-overlays.tar.zst" "$target/"
  echo "Source package prepared in $target (no build or install)"
  exit 0
fi
# Requires makepkg, Meson and GNOME build dependencies inside the test VM.
(cd "$work" && makepkg --cleanbuild --noconfirm --nodeps)
target="${1:-$root/dist/packages}"
mkdir -p "$target"
cp "$work"/"$pkgname"-*.pkg.tar.zst "$target/"
echo "Nodalix Shell package in $target."
