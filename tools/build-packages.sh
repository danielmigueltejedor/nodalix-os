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
tar --zstd -C "$root" -cf "$srcdir/nodalix-shell-$pkgver.tar.zst" \
  --exclude shell/blobs-plugin/build \
  --exclude shell/Caelestia \
  shell
tar --zstd -C "$root" -cf "$srcdir/nodalix-phone-link-$phone_pkgver.tar.zst" \
  --exclude phone-link/src/iphonebridge/.git \
  --exclude='*/__pycache__' \
  --exclude='*.pyc' \
  --exclude phone-link/tests \
  phone-link

tar --zstd -C "$root" -cf "$srcdir/nodalix-integrations-$pkgver.tar.zst" \
  shell/scripts shell/systemd packaging/nodalix-shell/is-hyprland-session
tar --zstd -C "$root" -cf "$srcdir/nodalix-gnome-$pkgver.tar.zst" \
  --exclude='*/__pycache__' --exclude='*.pyc' \
  --exclude=gnome/extensions/glocalsend@donnybeelo.github.com gnome

updater_dir="$workdir/nodalix-updater"
apps_dir="$workdir/nodalix-apps"
shell_dir="$workdir/nodalix-shell"
release_dir="$workdir/nodalix-release"
phone_link_dir="$workdir/nodalix-phone-link"
fluent_dir="$workdir/nodalix-fluent-emoji"
wallpaper_dir="$workdir/nodalix-wallpapers"
colloid_dir="$workdir/nodalix-colloid-icons"
cursor_dir="$workdir/nodalix-cursor-theme"
mkdir -p "$updater_dir" "$apps_dir" "$shell_dir" "$release_dir" "$phone_link_dir" "$fluent_dir" "$wallpaper_dir" "$colloid_dir" "$cursor_dir"

cp "$root/packaging/nodalix-updater/PKGBUILD" "$updater_dir/PKGBUILD"
cp "$srcdir/nodalix-updater-$pkgver.tar.zst" "$updater_dir/"
inject_sha256 "$updater_dir/PKGBUILD" "$updater_dir/nodalix-updater-$pkgver.tar.zst"

cp "$root/packaging/nodalix-apps/PKGBUILD" "$apps_dir/PKGBUILD"
cp "$srcdir/nodalix-apps-$pkgver.tar.zst" "$apps_dir/"
inject_sha256 "$apps_dir/PKGBUILD" "$apps_dir/nodalix-apps-$pkgver.tar.zst"

cp "$root/packaging/nodalix-shell/PKGBUILD" \
   "$root/packaging/nodalix-shell/nodalix-shell" \
   "$root/packaging/nodalix-shell/nodalix-shell.install" \
   "$root/packaging/nodalix-shell/nodalix-shell.service" \
   "$root/packaging/nodalix-shell/nodalix-app-accent.service" \
   "$root/packaging/nodalix-shell/nodalix-app-accent.path" \
   "$root/packaging/nodalix-shell/VERSION" \
   "$shell_dir/"
cp "$srcdir/nodalix-shell-$pkgver.tar.zst" "$shell_dir/"
inject_sha256 "$shell_dir/PKGBUILD" \
  "$shell_dir/nodalix-shell-$pkgver.tar.zst" \
  "$shell_dir/nodalix-shell" \
  "$shell_dir/nodalix-shell.service" \
  "$shell_dir/nodalix-app-accent.service" \
  "$shell_dir/nodalix-app-accent.path" \
  "$shell_dir/VERSION"

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
hymission_dir="$workdir/nodalix-hymission"
mkdir -p "$hymission_dir"
cp "$root/packaging/nodalix-hymission/PKGBUILD" "$hymission_dir/"

cp "$root/packaging/nodalix-wallpapers/PKGBUILD" \
   "$root/packaging/nodalix-wallpapers/SOURCES.md" \
   "$root/packaging/nodalix-wallpapers/"*.jpg \
   "$wallpaper_dir/"

greeter_dir="$workdir/nodalix-greeter-theme"
mkdir -p "$greeter_dir"
cp "$root/packaging/nodalix-greeter-theme/"* "$greeter_dir/"
inject_sha256 "$greeter_dir/PKGBUILD" \
  "$greeter_dir/regreet.css" \
  "$greeter_dir/regreet.toml" \
  "$greeter_dir/hyprland.lua" \
  "$greeter_dir/greetd-config.toml" \
  "$greeter_dir/nodalix-greeter-session"

engine_dir="$workdir/nodalix-wallpaper-engine"
mkdir -p "$engine_dir"
cp "$root/packaging/nodalix-wallpaper-engine/PKGBUILD" "$engine_dir/"

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
  python3 "$root/tools/build-settings-packages.py" --output "$root/dist/native-settings"
  find "$root/dist/native-settings" -mindepth 2 -maxdepth 2 -type f -name "*.pkg.tar.zst" ! -name "*-debug-*" -exec cp {} "$outdir/" \;
  makepkg_one "$engine_dir"
  makepkg_one "$updater_dir"
  makepkg_one "$apps_dir"
  makepkg_one "$integrations_dir"
  makepkg_one "$gnome_dir"
  makepkg_one "$shell_dir"
  makepkg_one "$phone_link_dir"
  makepkg_one "$fluent_dir"
  makepkg_one "$colloid_dir"
  makepkg_one "$hymission_dir"
  makepkg_one "$cursor_dir"
  makepkg_one "$greeter_dir"
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
