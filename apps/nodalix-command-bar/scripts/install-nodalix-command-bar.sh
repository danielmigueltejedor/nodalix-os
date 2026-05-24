#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOURCE="${ROOT_DIR}/apps/nodalix-command-bar/src/nodalix-command-bar"
WRAPPER="${ROOT_DIR}/local/bin/nodalix-command-bar"

if [[ ! -x "${SOURCE}" ]]; then
  chmod +x "${SOURCE}"
fi

mkdir -p "${ROOT_DIR}/local/bin/Backups"
if [[ -e "${WRAPPER}" ]]; then
  cp -a "${WRAPPER}" "${ROOT_DIR}/local/bin/Backups/nodalix-command-bar.bak.$(date +%Y%m%d-%H%M%S)"
fi

cat > "${WRAPPER}" <<'EOF'
#!/usr/bin/env bash
exec "$HOME/Projects/nodalix-os/apps/nodalix-command-bar/src/nodalix-command-bar" "$@"
EOF
chmod +x "${WRAPPER}"

echo "Installed Nodalix Command Bar wrapper: ${WRAPPER}"

