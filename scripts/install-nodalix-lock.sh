#!/usr/bin/env bash
# Install nodalix-lock and related wrappers into ~/.local/bin (no root required).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
REQUIRED=(nodalix-lock nodalix-system-lock nodalix-system-wake nodalix-validate-system)

mkdir -p "$BIN_DIR"

install_one() {
  local name="$1"
  local src="${ROOT}/local/bin/${name}"
  local dst="${BIN_DIR}/${name}"

  if [ ! -f "$src" ]; then
    echo "ERROR: falta ${src}" >&2
    return 1
  fi

  chmod 755 "$src"
  ln -sf "$src" "$dst"

  if [ ! -e "$dst" ]; then
    echo "ERROR: no se creó ${dst}" >&2
    return 1
  fi

  if [ ! -x "$dst" ]; then
    echo "ERROR: ${dst} no es ejecutable — prueba: chmod 755 ${src} && ln -sf ${src} ${dst}" >&2
    return 1
  fi

  echo "  OK   ${dst} -> $(readlink -f "$dst")"
}

echo "==> Instalando wrappers Nodalix en ${BIN_DIR}"

for name in "${REQUIRED[@]}"; do
  install_one "$name"
done

if ! echo "${PATH:-}" | tr ':' '\n' | grep -qx "${BIN_DIR}"; then
  echo
  echo "AVISO: ${BIN_DIR} no está en PATH."
  echo "Añade a ~/.zshrc:"
  echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo
echo "Verificación:"
if command -v nodalix-lock >/dev/null 2>&1; then
  echo "  command -v nodalix-lock -> $(command -v nodalix-lock)"
else
  echo "  WARN nodalix-lock no está en PATH de esta shell"
fi

if grep -q 'nodalix-lock.flock' "$(readlink -f "${BIN_DIR}/nodalix-lock")" 2>/dev/null; then
  echo "  OK   wrapper oficial (flock + pgrep)"
else
  echo "  WARN el wrapper instalado no parece el oficial del repo"
fi

test -x "${BIN_DIR}/nodalix-lock"
echo
echo "Listo. Prueba: nodalix-lock"
