#!/usr/bin/env bash
# Builds Archivador, Settings and System Monitor from this checkout and installs them over the
# installed ones, so a change can be tried as the real apps and not through `cargo run`.
#
#   tools/install-native-apps-local.sh
#
# It asks for the sudo password once, at the install. Nothing here is published.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
work=${NODALIX_LOCAL_BUILD:-$HOME/nodalix-build}
version=$(sed -n 's/^pkgver=//p' "$root/packaging/nodalix-native-apps/PKGBUILD")

echo "==> Building from $(git -C "$root" rev-parse --short HEAD) ($(git -C "$root" rev-parse --abbrev-ref HEAD))"
if [[ -n $(git -C "$root" status --porcelain -- apps packaging gnome/control-center/ndl) ]]; then
  echo "    (there are uncommitted changes under apps/, they are built too)"
fi
rm -rf "$work"
mkdir -p "$work"
cp "$root/packaging/nodalix-native-apps/PKGBUILD" "$work/"
tar --zstd -C "$root" -cf "$work/nodalix-native-apps-$version.tar.zst" \
  --exclude='apps/target' --transform='s,^gnome/control-center/ndl/,ndl/,' \
  apps gnome/control-center/ndl/LICENSE
(cd "$work" && makepkg -f --noconfirm --nodeps --cleanbuild)

echo "==> Closing the running apps"
pkill -f '^/usr/bin/nodalix-archivador'  || true
pkill -f '^/usr/bin/nodalix-settings'  || true
pkill -f '^/usr/bin/nodalix-monitor'  || true

echo "==> Installing"
sudo pacman -U --noconfirm \
  "$work/nodalix-archivador-$version-1-x86_64.pkg.tar.zst" \
  "$work/nodalix-ajustes-$version-1-x86_64.pkg.tar.zst" \
  "$work/nodalix-monitor-$version-1-x86_64.pkg.tar.zst"

echo "==> Installed binaries"
ls -l --time-style=full-iso /usr/bin/nodalix-archivador /usr/bin/nodalix-settings
echo "Open Archivador again from the launcher. If it still looks like the old one, the launcher is"
echo "starting something else: run \`pgrep -af archivador\` and \`which -a nodalix-archivador nautilus\`."
