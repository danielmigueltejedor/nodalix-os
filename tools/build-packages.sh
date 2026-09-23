#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
version=$(tr -d '[:space:]' < "$root/VERSION")
pkgver=${version//-/}
pkgrel=1
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
tar --zstd -C "$root" -cf "$srcdir/nodalix-shell-$pkgver.tar.zst" \
  --exclude shell/blobs-plugin/build \
  --exclude shell/Caelestia \
  shell
tar --zstd -C "$root" -cf "$srcdir/nodalix-phone-link-$pkgver.tar.zst" \
  --exclude phone-link/src/iphonebridge/.git \
  --exclude phone-link/src/iphonebridge/__pycache__ \
  --exclude phone-link/tests \
  phone-link

updater_dir="$workdir/nodalix-updater"
shell_dir="$workdir/nodalix-shell"
release_dir="$workdir/nodalix-release"
phone_link_dir="$workdir/nodalix-phone-link"
fluent_dir="$workdir/nodalix-fluent-emoji"
wallpaper_dir="$workdir/nodalix-wallpapers"
colloid_dir="$workdir/nodalix-colloid-icons"
mkdir -p "$updater_dir" "$shell_dir" "$release_dir" "$phone_link_dir" "$fluent_dir" "$wallpaper_dir" "$colloid_dir"

cp "$root/packaging/nodalix-updater/PKGBUILD" "$updater_dir/PKGBUILD"
cp "$srcdir/nodalix-updater-$pkgver.tar.zst" "$updater_dir/"
inject_sha256 "$updater_dir/PKGBUILD" "$updater_dir/nodalix-updater-$pkgver.tar.zst"

cp "$root/packaging/nodalix-shell/PKGBUILD" \
   "$root/packaging/nodalix-shell/nodalix-shell" \
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
cp "$srcdir/nodalix-phone-link-$pkgver.tar.zst" "$phone_link_dir/"
inject_sha256 "$phone_link_dir/PKGBUILD" "$phone_link_dir/nodalix-phone-link-$pkgver.tar.zst"

cp "$root/packaging/nodalix-fluent-emoji/PKGBUILD" \
   "$root/packaging/nodalix-fluent-emoji/75-nodalix-fluent-emoji.conf" \
   "$fluent_dir/"

cp "$root/packaging/nodalix-colloid-icons/PKGBUILD" "$colloid_dir/"
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
inject_sha256 "$greeter_dir/PKGBUILD" "$greeter_dir/regreet.css"

engine_dir="$workdir/nodalix-wallpaper-engine"
mkdir -p "$engine_dir"
cp "$root/packaging/nodalix-wallpaper-engine/PKGBUILD" "$engine_dir/"

makepkg_one() {
  local dir=$1
  (cd "$dir" && makepkg -f --noconfirm --nodeps --cleanbuild)
}

if [[ ${NODALIX_SKIP_MAKEPKG:-0} != 1 ]]; then
  makepkg_one "$engine_dir"
  makepkg_one "$updater_dir"
  makepkg_one "$shell_dir"
  makepkg_one "$phone_link_dir"
  makepkg_one "$fluent_dir"
  makepkg_one "$colloid_dir"
  makepkg_one "$hymission_dir"
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
