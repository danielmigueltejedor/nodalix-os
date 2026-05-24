#!/usr/bin/env bash
set -euo pipefail

FALLBACK="/usr/local/share/nodalix-greeter/config.regreet-fallback.toml"
GREETD_CONFIG="/etc/greetd/config.toml"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root: sudo scripts/rollback-regreet.sh" >&2
  exit 1
fi

if [[ ! -f "${FALLBACK}" ]]; then
  echo "Missing fallback config: ${FALLBACK}" >&2
  echo "Reinstall nodalix-greeter assets or copy config/greetd/config.regreet-fallback.toml manually." >&2
  exit 1
fi

if [[ -f "${GREETD_CONFIG}" ]]; then
  backup="${GREETD_CONFIG}.backup.$(date +%Y%m%d-%H%M%S)"
  cp -a "${GREETD_CONFIG}" "${backup}"
  echo "Backed up existing greetd config to ${backup}"
fi

install -Dm644 "${FALLBACK}" "${GREETD_CONFIG}"
echo "Restored ReGreet fallback greetd config."
echo "Restart greetd from a safe TTY when ready."

