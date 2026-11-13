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
components="$root/release/components.json"

kernel_version=$(python3 - "$components" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream:
    rows = json.load(stream)["components"]
print(next((row["version"] for row in rows if row["package"] == "linux-nodalix"), ""))
PYTHON
)
if [[ -n "$kernel_version" ]]; then
  kernel_asset=${NODALIX_KERNEL_ASSET:?build the Nodalix kernel and supply its package in NODALIX_KERNEL_ASSET}
  read -r package_name package_version < <(pacman -Qp -- "$kernel_asset")
  if [[ "$package_name" != linux-nodalix || "${package_version%-*}" != "$kernel_version" ]]; then
    printf 'Kernel package does not match the release component\n' >&2
    exit 1
  fi
fi

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
tar --zstd -C "$root" -cf "$srcdir/nodalix-nxpm-$pkgver.tar.zst" \
  --exclude='*/__pycache__' --exclude='*.pyc' --exclude='nxpm/tests' --exclude='nxpm/.pytest_cache' \
  nxpm docs/nxpm.md
tar --zstd -C "$root" -cf "$srcdir/nodalix-phone-link-$phone_pkgver.tar.zst" \
  --exclude phone-link/src/iphonebridge/.git \
  --exclude='*/__pycache__' \
  --exclude='*.pyc' \
  --exclude phone-link/tests \
  phone-link

tar --zstd -C "$root" -cf "$srcdir/nodalix-integrations-$pkgver.tar.zst" \
  --exclude='*/__pycache__' --exclude='*.pyc' \
  integrations
tar --zstd -C "$root" -cf "$srcdir/nodalix-gnome-$pkgver.tar.zst" \
  --exclude='*/__pycache__' --exclude='*.pyc' \
  --exclude=gnome/settings/animated gnome

updater_dir="$workdir/nodalix-updater"
apps_dir="$workdir/nodalix-apps"
nxpm_dir="$workdir/nodalix-nxpm"
release_dir="$workdir/nodalix-release"
phone_link_dir="$workdir/nodalix-phone-link"
fluent_dir="$workdir/nodalix-fluent-emoji"
wallpaper_dir="$workdir/nodalix-wallpapers"
colloid_dir="$workdir/nodalix-colloid-icons"
cursor_dir="$workdir/nodalix-cursor-theme"
mkdir -p "$updater_dir" "$apps_dir" "$nxpm_dir" "$release_dir" "$phone_link_dir" "$fluent_dir" "$wallpaper_dir" "$colloid_dir" "$cursor_dir"

cp "$root/packaging/nodalix-updater/PKGBUILD" "$root/packaging/nodalix-updater/nodalix-updater.install" "$updater_dir/"
cp "$srcdir/nodalix-updater-$pkgver.tar.zst" "$updater_dir/"
inject_sha256 "$updater_dir/PKGBUILD" "$updater_dir/nodalix-updater-$pkgver.tar.zst"

cp "$root/packaging/nodalix-apps/PKGBUILD" "$apps_dir/PKGBUILD"
cp "$srcdir/nodalix-apps-$pkgver.tar.zst" "$apps_dir/"
inject_sha256 "$apps_dir/PKGBUILD" "$apps_dir/nodalix-apps-$pkgver.tar.zst"

cp "$root/packaging/nodalix-nxpm/PKGBUILD" "$nxpm_dir/PKGBUILD"
cp "$srcdir/nodalix-nxpm-$pkgver.tar.zst" "$nxpm_dir/"
inject_sha256 "$nxpm_dir/PKGBUILD" "$nxpm_dir/nodalix-nxpm-$pkgver.tar.zst"

keyring_dir="$workdir/nodalix-keyring"
mkdir -p "$keyring_dir"
cp "$root/packaging/nodalix-keyring/"{PKGBUILD,nodalix-keyring.install,nodalix-repo-key.pgp} "$keyring_dir/"
inject_sha256 "$keyring_dir/PKGBUILD" "$keyring_dir/nodalix-repo-key.pgp"

store_dir="$workdir/nodalix-store"
mkdir -p "$store_dir"
cp "$root/packaging/nodalix-store/PKGBUILD" "$store_dir/"

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
find "$root/packaging/nodalix-cursor-theme" -maxdepth 1 -type f \
  -exec cp {} "$cursor_dir/" \;
cp "$root/packaging/nodalix-wallpapers/PKGBUILD" \
   "$root/packaging/nodalix-wallpapers/SOURCES.md" \
   "$root/packaging/nodalix-wallpapers/"*.jpg \
   "$wallpaper_dir/"

# Build a pinned Nodalix package instead of relying on mutable AUR updates.
xpadneo_dir="$workdir/nodalix-xpadneo-dkms"
mkdir -p "$xpadneo_dir"
cp "$root/packaging/nodalix-xpadneo-dkms/"{PKGBUILD,preserve-ble-identity.patch,99-nodalix-xpadneo-real-id.conf} "$xpadneo_dir/"

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
    python3 "$root/tools/build-settings-packages.py" --skip-control-center --output "$root/dist/native-settings"
  fi
  find "$root/dist/native-settings" -mindepth 2 -maxdepth 2 -type f -name "*.pkg.tar.zst" ! -name "*-debug-*" -exec cp {} "$outdir/" \;
  makepkg_one "$updater_dir"
  makepkg_one "$apps_dir"
  makepkg_one "$nxpm_dir"
  makepkg_one "$integrations_dir"
  rounded_dir="$workdir/gnome-rounded-blur"
  mkdir -p "$rounded_dir"
  cp "$root/packaging/gnome-rounded-blur/PKGBUILD" "$rounded_dir/"
  tar --zstd -C "$root/gnome/lib" --transform='s,^rounded-blur,gnome-rounded-blur,' \
    -cf "$rounded_dir/gnome-rounded-blur-1.0.0.tar.zst" rounded-blur
  inject_sha256 "$rounded_dir/PKGBUILD" "$rounded_dir/gnome-rounded-blur-1.0.0.tar.zst"
  makepkg_one "$rounded_dir"
  makepkg_one "$gnome_dir"
  bash "$root/tools/build-nodalix-shell.sh" "$outdir"
  makepkg_one "$phone_link_dir"
  ofono_dir="$workdir/nodalix-ofono"
  mkdir -p "$ofono_dir"
  cp "$root/packaging/nodalix-ofono/PKGBUILD" "$ofono_dir/"
  makepkg_one "$ofono_dir"
  makepkg_one "$xpadneo_dir"
  makepkg_one "$fluent_dir"
  makepkg_one "$colloid_dir"
  makepkg_one "$cursor_dir"
  makepkg_one "$wallpaper_dir"
  # The store is built last: its Vala and Blueprint toolchain must not be visible to the
  # earlier Meson builds, which probe for blueprint-compiler and change behaviour if it exists.
  if [[ ${NODALIX_CI_CONTAINER:-} == 1 ]]; then
    sudo pacman -S --needed --noconfirm vala blueprint-compiler gettext libxml2 desktop-file-utils \
      python python-babel perl glib2-devel gtk4 libadwaita libdex flatpak libxmlb libyaml libsoup3 \
      json-glib glycin glycin-gtk4 webkitgtk-6.0 libsecret libproxy malcontent gtksourceview5 md4c \
      appstream dbus
  fi
  makepkg_one "$store_dir"
  if [[ ${NODALIX_SKIP_NATIVE_BUILD:-0} != 1 ]]; then
    # The native apps (Settings, System Monitor, Archivador: Rust on the Nodalia Design Language)
    # replace nodalix-control-center and nautilus. Brújula, the browser, is built from a pinned
    # release commit. Both are part of the release set from 0.6.0 (docs/preparation-0.6.0.md).
    native_dir="$workdir/nodalix-native-apps"
    mkdir -p "$native_dir"
    cp "$root/packaging/nodalix-native-apps/PKGBUILD" "$native_dir/"
    tar --zstd -C "$root" -cf "$native_dir/nodalix-native-apps-$pkgver.tar.zst" \
      --exclude='apps/target' --transform='s,^gnome/control-center/ndl/,ndl/,' \
      apps gnome/control-center/ndl/LICENSE
    inject_sha256 "$native_dir/PKGBUILD" "$native_dir/nodalix-native-apps-$pkgver.tar.zst"
    makepkg_one "$native_dir"
    brujula_dir="$workdir/nodalix-brujula"
    mkdir -p "$brujula_dir"
    cp "$root/packaging/nodalix-brujula/PKGBUILD" "$brujula_dir/"
    makepkg_one "$brujula_dir"
  fi
  makepkg_one "$keyring_dir"
  makepkg_one "$release_dir"
  find "$workdir" -mindepth 2 -maxdepth 2 -type f -name '*.pkg.tar.zst' ! -name '*-debug-*' -exec cp {} "$outdir/" \;
fi

if [[ -n "$kernel_version" && ! "$kernel_asset" -ef "$outdir/${kernel_asset##*/}" ]]; then
  cp -- "$kernel_asset" "$outdir/"
fi

(
  cd -- "$outdir"
  manifest_tmp=$(mktemp .nodalix-manifest.XXXXXX)
  trap 'unlink "$manifest_tmp"' EXIT
  python3 "$root/tools/generate-manifest.py" > "$manifest_tmp"
  chmod 644 "$manifest_tmp"
  mv -fT -- "$manifest_tmp" nodalix-manifest.json
  trap - EXIT
)

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
