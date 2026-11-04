#!/usr/bin/env bash
# Assemble the source package in an isolated Arch VM or clean chroot.
# This script never installs the preview on the host.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
version=$(tr -d '[:space:]' < "$root/VERSION")
pkgver=$(sed -n 's/^pkgver=//p' "$root/packaging/nodalix-shell-preview/PKGBUILD")
if [[ "$version" != "$pkgver" ]]; then
  echo "Nodalix Shell package $pkgver does not match Nodalix OS $version" >&2
  exit 1
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$root/packaging/nodalix-shell-preview/PKGBUILD" "$work/PKGBUILD"
tar --zstd -C "$root" -cf "$work/nodalix-shell-overlays.tar.zst" \
  gnome/nodalix-shell tools/prepare-nodalix-shell.py
sum=$(sha256sum "$work/nodalix-shell-overlays.tar.zst" | cut -d' ' -f1)
sed -i "s/AUTO_OVERLAY_SHA256/$sum/" "$work/PKGBUILD"
if [[ "${1:-}" == '--prepare-only' ]]; then
  target="$root/dist/nodalix-shell-preview-source"
  mkdir -p "$target"
  cp "$work/PKGBUILD" "$work/nodalix-shell-overlays.tar.zst" "$target/"
  echo "Source package prepared in $target (no build or install)"
  exit 0
fi
# Requires makepkg, Meson and GNOME build dependencies inside the test VM.
(cd "$work" && makepkg --cleanbuild --noconfirm)
target="${1:-$root/dist/packages}"
mkdir -p "$target"
cp "$work"/nodalix-shell-preview-*.pkg.tar.zst "$target/"
echo "Preview package in $target. Installation is NOT automatic."
