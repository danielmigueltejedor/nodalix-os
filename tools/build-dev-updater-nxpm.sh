#!/usr/bin/env bash
# Build local candidate packages for nodalix-updater and nodalix-nxpm only.
# Intended for disposable testing on prepare/0.5.1 before merge.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd -- "$root"

version=$(tr -d '[:space:]' < VERSION)
outdir="$root/dist/dev"
srcdir="$outdir/src"
builddir="$outdir/build"
pkgdir="$outdir/packages"

mkdir -p "$srcdir" "$builddir" "$pkgdir"

tar --zstd -C "$root" -cf "$srcdir/nodalix-updater-$version.tar.zst" updater
# PKGBUILD installs from $srcdir/nxpm/... and $srcdir/docs/nxpm.md
tar --zstd -C "$root" \
  --exclude='nxpm/.venv' \
  --exclude='nxpm/**/__pycache__' \
  --exclude='nxpm/**/*.egg-info' \
  -cf "$srcdir/nodalix-nxpm-$version.tar.zst" nxpm docs/nxpm.md

build_one() {
  local name=$1
  local extra_install=${2:-}
  local dir="$builddir/$name"
  rm -rf "$dir"
  mkdir -p "$dir"
  cp "$root/packaging/$name/PKGBUILD" "$dir/"
  if [[ -n "$extra_install" ]]; then
    cp "$root/packaging/$name/$extra_install" "$dir/"
  fi
  cp "$srcdir/$name-$version.tar.zst" "$dir/"
  # Local candidate builds skip pinned source checksums.
  sed -i "s/^sha256sums=.*/sha256sums=('SKIP')/" "$dir/PKGBUILD"
  (cd "$dir" && makepkg -f --noconfirm --nodeps --cleanbuild)
  find "$dir" -maxdepth 1 -type f -name '*.pkg.tar.zst' ! -name '*-debug-*' -exec cp -f {} "$pkgdir/" \;
}

build_one nodalix-updater nodalix-updater.install
build_one nodalix-nxpm

echo
echo "Packages in $pkgdir:"
ls -1 "$pkgdir"/*.pkg.tar.zst
echo
echo "Install with:"
echo "  sudo pacman -U $pkgdir/nodalix-updater-*.pkg.tar.zst $pkgdir/nodalix-nxpm-*.pkg.tar.zst"
