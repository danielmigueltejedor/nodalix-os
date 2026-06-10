#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Instalando Nodalix OS layer..."

install_packages_from_file() {
  local file="$1"

  if [ -f "$file" ]; then
    echo "==> Instalando paquetes desde $file"
    sudo pacman -S --needed --noconfirm $(grep -vE "^\s*#|^\s*$" "$file")
  fi
}

backup_path() {
  local path="$1"

  if [ -e "$path" ] && [ ! -L "$path" ]; then
    local backup="${path}.backup.$(date +%F-%H%M%S)"
    echo "==> Backup: $path -> $backup"
    mv "$path" "$backup"
  fi
}

link_config() {
  local src="$1"
  local dst="$2"

  mkdir -p "$(dirname "$dst")"
  backup_path "$dst"
  ln -sfn "$src" "$dst"
}

echo "==> Actualizando base de paquetes..."
sudo pacman -Syu --needed --noconfirm

install_packages_from_file "$ROOT/packages/base.txt"
install_packages_from_file "$ROOT/packages/desktop.txt"
install_packages_from_file "$ROOT/packages/gaming.txt"
install_packages_from_file "$ROOT/packages/dev.txt"
install_packages_from_file "$ROOT/packages/privacy.txt"

echo "==> Enlazando configuración Nodalix..."

mkdir -p "$HOME/.config"
mkdir -p "$HOME/.local/bin"

link_config "$ROOT/config/hypr" "$HOME/.config/hypr"
link_config "$ROOT/config/waybar" "$HOME/.config/waybar"
link_config "$ROOT/config/wpe-hypr" "$HOME/.config/wpe-hypr"

if [ -d "$ROOT/config/hyprlock" ] && [ "$(ls -A "$ROOT/config/hyprlock" 2>/dev/null)" ]; then
  link_config "$ROOT/config/hyprlock" "$HOME/.config/hyprlock"
fi

if [ -d "$ROOT/config/hypridle" ] && [ "$(ls -A "$ROOT/config/hypridle" 2>/dev/null)" ]; then
  link_config "$ROOT/config/hypridle" "$HOME/.config/hypridle"
fi

if [ -d "$ROOT/config/mako" ] && [ "$(ls -A "$ROOT/config/mako" 2>/dev/null)" ]; then
  link_config "$ROOT/config/mako" "$HOME/.config/mako"
fi

if [ -d "$ROOT/config/nodalix" ]; then
  link_config "$ROOT/config/nodalix" "$HOME/.config/nodalix"
fi

echo "==> Instalando entradas de sesión Wayland (usuario local)..."

mkdir -p "$HOME/.local/share/wayland-sessions" "$HOME/.local/share/applications"
for session in nodalix.desktop hyprland-uwsm.desktop; do
  if [ -f "$ROOT/local/share/wayland-sessions/$session" ]; then
    ln -sfn "$ROOT/local/share/wayland-sessions/$session" \
      "$HOME/.local/share/wayland-sessions/$session"
  fi
done
if [ -f "$ROOT/local/share/applications/hyprland-nodalix.desktop" ]; then
  ln -sfn "$ROOT/local/share/applications/hyprland-nodalix.desktop" \
    "$HOME/.local/share/applications/hyprland-nodalix.desktop"
fi

echo "==> Enlazando scripts..."

if [ -x "$ROOT/scripts/install-nodalix-lock.sh" ]; then
  "$ROOT/scripts/install-nodalix-lock.sh"
else
  mkdir -p "$HOME/.local/bin"
  for f in "$ROOT"/local/bin/*; do
    [ -f "$f" ] || continue
    chmod 755 "$f"
    ln -sf "$f" "$HOME/.local/bin/$(basename "$f")"
  done
fi

if [ ! -x "$HOME/.local/bin/nodalix-lock" ]; then
  echo "ERROR: ~/.local/bin/nodalix-lock no instalado o no ejecutable." >&2
  echo "Ejecuta: $ROOT/scripts/install-nodalix-lock.sh" >&2
  exit 1
fi

echo "==> Validando bloqueo e idle..."
if [ -x "$ROOT/local/bin/nodalix-validate-system" ]; then
  "$ROOT/local/bin/nodalix-validate-system" || true
fi

echo "==> Activando servicios básicos..."

sudo systemctl enable --now NetworkManager.service || true
systemctl --user daemon-reload || true
systemctl --user disable --now nodalix-idle.service 2>/dev/null || true

echo
echo "Nodalix OS layer instalado."
echo "Reinicia sesión o ejecuta: hyprctl reload"
