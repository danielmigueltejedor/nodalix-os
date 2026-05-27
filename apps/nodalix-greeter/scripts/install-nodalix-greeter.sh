#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="$(cd "${ROOT_DIR}/../.." && pwd)"
BIN_PATH="/usr/local/bin/nodalix-greeter"
SHARE_DIR="/usr/local/share/nodalix-greeter"
BRAND_DIR="/usr/share/nodalix/brand"
CONFIG_DIR="/etc/nodalix/greeter"
CONFIG_PATH="${CONFIG_DIR}/config.toml"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root: sudo scripts/install-nodalix-greeter.sh" >&2
  exit 1
fi

cd "${ROOT_DIR}"
cargo build --release

install -Dm755 "target/release/nodalix-greeter" "${BIN_PATH}"
install -Dm644 "data/nodalix-greeter.css" "${SHARE_DIR}/nodalix-greeter.css"
install -Dm644 "data/nodalix-greeter.dev.toml" "${SHARE_DIR}/config.example.toml"
install -Dm644 "config/greetd/hyprland-nodalix-greeter.conf" "/usr/local/share/nodalix-greeter/hyprland-nodalix-greeter.conf"
install -Dm644 "config/greetd/config.nodalix-greeter.toml" "/usr/local/share/nodalix-greeter/config.nodalix-greeter.toml"
install -Dm644 "config/greetd/config.regreet-fallback.toml" "/usr/local/share/nodalix-greeter/config.regreet-fallback.toml"
install -Dm644 "${REPO_DIR}/assets/brand/nodalix-logo-symbol.svg" "${BRAND_DIR}/nodalix-logo-symbol.svg"
install -Dm644 "${REPO_DIR}/assets/brand/nodalix-logo.svg" "${BRAND_DIR}/nodalix-logo.svg"
install -Dm644 "${REPO_DIR}/local/share/wayland-sessions/nodalix.desktop" \
  /usr/local/share/wayland-sessions/nodalix.desktop
install -Dm644 "${REPO_DIR}/local/share/wayland-sessions/hyprland-uwsm.desktop" \
  /usr/local/share/wayland-sessions/hyprland-uwsm.desktop
install -Dm644 "${REPO_DIR}/local/share/applications/hyprland-nodalix.desktop" \
  /usr/local/share/applications/hyprland-nodalix.desktop
chmod 755 "/usr/share/nodalix" "${BRAND_DIR}"
chmod 644 "${BRAND_DIR}/nodalix-logo-symbol.svg" "${BRAND_DIR}/nodalix-logo.svg"

mkdir -p "${CONFIG_DIR}"
if [[ ! -e "${CONFIG_PATH}" ]]; then
  install -m644 "data/nodalix-greeter.dev.toml" "${CONFIG_PATH}"
  echo "Installed default config: ${CONFIG_PATH}"
else
  echo "Keeping existing config: ${CONFIG_PATH}"
fi

echo
echo "Installed nodalix-greeter to ${BIN_PATH}"
echo "Installed Nodalix brand assets to ${BRAND_DIR}"
echo
echo "Manual next steps:"
echo "  1. Copy /usr/local/share/nodalix-greeter/hyprland-nodalix-greeter.conf to /etc/greetd/ if desired."
echo "  2. Back up /etc/greetd/config.toml."
echo "  3. Test using the sample config:"
echo "     /usr/local/share/nodalix-greeter/config.nodalix-greeter.toml"
echo "  4. Keep /usr/local/share/nodalix-greeter/config.regreet-fallback.toml for rollback."
echo "  5. If /etc/nodalix/greeter/config.toml already exists, set:"
echo "     session_command = \"uwsm start nodalix.desktop\""
