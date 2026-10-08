#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
version=$(tr -d '[:space:]' < "$root/VERSION")
pkgver=${version//-/}
phone_pkgver=$(sed -n 's/^pkgver=//p' "$root/packaging/nodalix-phone-link/PKGBUILD")
outdir=${1:-"$root/dist/packages"}
srcdir="$root/dist/src"
workdir="$root/dist/build"
cachedir="$root/dist/cache"

rm -rf "$srcdir" "$workdir"
mkdir -p "$outdir" "$srcdir" "$workdir" "$cachedir"
export SRCDEST="$cachedir"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

inject_sha256() {
  local pkgbuild=$1
  shift
  local sums=()
  local file
  for file in "$@"; do
    sums+=("'$(sha256_file "$file")'")
  done
  local joined
  joined=$(IFS=' '; echo "${sums[*]}")
  python3 - "$pkgbuild" "$joined" <<'PY'
import pathlib, re, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
replacement = "sha256sums=(" + sys.argv[2] + ")"
text = re.sub(r"sha256sums=\([^)]*\)", replacement, text, count=1)
path.write_text(text)
PY
}

echo "Building Nodalix $version (pkgver=$pkgver)"

tar --zstd -C "$root" -cf "$srcdir/nodalix-updater-$pkgver.tar.zst" updater
tar --zstd -C "$root" -cf "$srcdir/nodalix-apps-$pkgver.tar.zst" nodalix-apps
tar --zstd -C "$root" -cf "$srcdir/nodalix-phone-link-$phone_pkgver.tar.zst" \
  --exclude phone-link/src/iphonebridge/.git \
  --exclude='*/__pycache__' \
  --exclude='*.pyc' \
  --exclude phone-link/tests \
  phone-link

tar --zstd -C "$root" -cf "$srcdir/nodalix-integrations-$pkgver.tar.zst" \
  integrations
tar --zstd -C "$root" -cf "$srcdir/nodalix-gnome-$pkgver.tar.zst" \
  --exclude='*/__pycache__' --exclude='*.pyc' \
  --exclude=gnome/settings/animated gnome

updater_dir="$workdir/nodalix-updater"
apps_dir="$workdir/nodalix-apps"
release_dir="$workdir/nodalix-release"
phone_link_dir="$workdir/nodalix-phone-link"
fluent_dir="$workdir/nodalix-fluent-emoji"
wallpaper_dir="$workdir/nodalix-wallpapers"
colloid_dir="$workdir/nodalix-colloid-icons"
cursor_dir="$workdir/nodalix-cursor-theme"
mkdir -p "$updater_dir" "$apps_dir" "$release_dir" "$phone_link_dir" "$fluent_dir" "$wallpaper_dir" "$colloid_dir" "$cursor_dir"

cp "$root/packaging/nodalix-updater/PKGBUILD" "$root/packaging/nodalix-updater/nodalix-updater.install" "$updater_dir/"
cp "$srcdir/nodalix-updater-$pkgver.tar.zst" "$updater_dir/"
inject_sha256 "$updater_dir/PKGBUILD" "$updater_dir/nodalix-updater-$pkgver.tar.zst"

cp "$root/packaging/nodalix-apps/PKGBUILD" "$apps_dir/PKGBUILD"
cp "$srcdir/nodalix-apps-$pkgver.tar.zst" "$apps_dir/"
inject_sha256 "$apps_dir/PKGBUILD" "$apps_dir/nodalix-apps-$pkgver.tar.zst"

cp "$root/packaging/nodalix-release/PKGBUILD" "$root/packaging/nodalix-release/VERSION" "$release_dir/"
inject_sha256 "$release_dir/PKGBUILD" "$release_dir/VERSION"

cp "$root/packaging/nodalix-phone-link/PKGBUILD" "$phone_link_dir/PKGBUILD"
cp "$root/packaging/nodalix-phone-link/nodalix-phone-link.install" "$phone_link_dir/nodalix-phone-link.install"
cp "$srcdir/nodalix-phone-link-$phone_pkgver.tar.zst" "$phone_link_dir/"
inject_sha256 "$phone_link_dir/PKGBUILD" "$phone_link_dir/nodalix-phone-link-$phone_pkgver.tar.zst"

cp "$root/packaging/nodalix-fluent-emoji/PKGBUILD" \
   "$root/packaging/nodalix-fluent-emoji/75-nodalix-fluent-emoji.conf" \
   "$fluent_dir/"

cp "$root/packaging/nodalix-colloid-icons/PKGBUILD" "$colloid_dir/"
cp "$root/packaging/nodalix-cursor-theme/"* "$cursor_dir/"
cp "$root/packaging/nodalix-wallpapers/PKGBUILD" \
   "$root/packaging/nodalix-wallpapers/SOURCES.md" \
   "$root/packaging/nodalix-wallpapers/"*.jpg \
   "$wallpaper_dir/"

integrations_dir="$workdir/nodalix-integrations"
gnome_dir="$workdir/nodalix-gnome"
for name in integrations gnome; do
  dir="$workdir/nodalix-$name"
  mkdir -p "$dir"
  cp "$root/packaging/nodalix-$name/PKGBUILD" "$dir/"
  if [[ -f "$root/packaging/nodalix-$name/nodalix-$name.install" ]]; then
    cp "$root/packaging/nodalix-$name/nodalix-$name.install" "$dir/"
  fi
  cp "$srcdir/nodalix-$name-$pkgver.tar.zst" "$dir/"
  inject_sha256 "$dir/PKGBUILD" "$dir/nodalix-$name-$pkgver.tar.zst"
done

makepkg_one() {
  local dir=$1
  (cd "$dir" && makepkg -f --noconfirm --nodeps --cleanbuild)
}

if [[ ${NODALIX_SKIP_MAKEPKG:-0} != 1 ]]; then
  if [[ ${NODALIX_SKIP_NATIVE_BUILD:-0} != 1 ]]; then
    python3 "$root/tools/build-settings-packages.py" --output "$root/dist/native-settings"
  fi
  find "$root/dist/native-settings" -mindepth 2 -maxdepth 2 -type f -name "*.pkg.tar.zst" ! -name "*-debug-*" -exec cp {} "$outdir/" \;
  makepkg_one "$updater_dir"
  makepkg_one "$apps_dir"
  makepkg_one "$integrations_dir"
  rounded_dir="$workdir/gnome-rounded-blur"
  mkdir -p "$rounded_dir"
  cp "$root/packaging/gnome-rounded-blur/PKGBUILD" "$rounded_dir/"
  makepkg_one "$rounded_dir"
  makepkg_one "$gnome_dir"
  makepkg_one "$phone_link_dir"
  ofono_dir="$workdir/nodalix-ofono"
  mkdir -p "$ofono_dir"
  cp "$root/packaging/nodalix-ofono/PKGBUILD" "$ofono_dir/"
  makepkg_one "$ofono_dir"
  makepkg_one "$fluent_dir"
  makepkg_one "$colloid_dir"
  makepkg_one "$cursor_dir"
  makepkg_one "$wallpaper_dir"
  makepkg_one "$release_dir"
  find "$workdir" -mindepth 2 -maxdepth 2 -type f -name '*.pkg.tar.zst' ! -name '*-debug-*' -exec cp {} "$outdir/" \;
fi

python3 "$root/tools/generate-manifest.py" \
  --version-file "$root/VERSION" \
  --components "$root/release/components.json" \
  --assets-dir "$outdir" \
  --output "$outdir/nodalix-manifest.json"

python3 - "$outdir" <<'PY'
import hashlib, pathlib, sys
root = pathlib.Path(sys.argv[1])
lines = []
for path in sorted(root.glob("*")):
    if path.is_file() and path.name != "SHA256SUMS":
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        lines.append(f"{digest}  {path.name}")
(root / "SHA256SUMS").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
PY

echo "Packages written to $outdir"
