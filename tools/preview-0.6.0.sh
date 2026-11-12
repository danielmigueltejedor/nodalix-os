#!/usr/bin/env bash
# Prueba local de las apps de Nodalix 0.6.0 (Monitor del sistema, Ajustes y Brújula),
# sin tocar el sistema: todo va a ~/.local, sin root y sin reemplazar nada de GNOME.
#
#   tools/preview-0.6.0.sh check              comprueba dependencias y versiones
#   tools/preview-0.6.0.sh backup             guarda el estado actual en ~/Respaldos
#   tools/preview-0.6.0.sh install            compila e instala las tres apps en ~/.local
#   tools/preview-0.6.0.sh default-browser on|off   Brújula como navegador predeterminado
#   tools/preview-0.6.0.sh uninstall          lo quita todo y restaura el navegador
#   tools/preview-0.6.0.sh archivador         construye el paquete de Archivador (sin instalarlo)
#   tools/preview-0.6.0.sh store              construye la Tienda con NDL (sin instalarla)
#
# Archivador y la Tienda reemplazan paquetes del sistema (nautilus, nodalix-store 0.5.0), así que
# este script solo los construye y dice el comando de pacman, con su vuelta atrás.
set -euo pipefail

root=$(cd -- "$(dirname -- "$0")/.." && pwd)
state="${XDG_DATA_HOME:-$HOME/.local/share}/nodalix-preview"
manifest="$state/manifest"
bin="$HOME/.local/bin"
apps_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
brujula_dir="${BRUJULA_DIR:-$(dirname -- "$root")/brujula}"
brujula_url="${BRUJULA_URL:-https://github.com/danielmigueltejedor/brujula.git}"
backup_dir="$HOME/Respaldos/nodalix-pre-0.6.0-preview-$(date +%F)"
brujula_desktop=dev.danielmiguelt.Brujula.desktop

die() { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "falta '$1' ($2)"; }

check() {
  need cargo "sudo pacman -S rust"
  need git "sudo pacman -S git"
  need pkg-config "sudo pacman -S pkgconf"
  need update-desktop-database "sudo pacman -S desktop-file-utils"
  local gtk web
  gtk=$(pkg-config --modversion gtk4 2>/dev/null) || die "falta gtk4 (sudo pacman -S gtk4)"
  web=$(pkg-config --modversion webkitgtk-6.0 2>/dev/null) || die "falta webkitgtk-6.0 (sudo pacman -S webkitgtk-6.0)"
  [[ $(printf '%s\n4.20\n' "$gtk" | sort -V | head -1) == 4.20 ]] || die "GTK $gtk es anterior a 4.20"
  echo "GTK $gtk, WebKitGTK $web, $(cargo --version)"
  echo "Nodalix-os: $(git -C "$root" rev-parse --short HEAD) ($(git -C "$root" rev-parse --abbrev-ref HEAD))"
  pacman -Q nodalix-gnome nodalix-settings nodalix-store linux-nodalix 2>/dev/null || true
}

backup() {
  mkdir -p "$backup_dir"
  pacman -Q > "$backup_dir/paquetes.txt"
  xdg-settings get default-web-browser > "$backup_dir/navegador-predeterminado.txt" 2>/dev/null || true
  cp -a "$HOME/.config/mimeapps.list" "$backup_dir/" 2>/dev/null || true
  [[ -f $manifest ]] && cp -a "$manifest" "$backup_dir/manifest-previo" || true
  echo "Respaldo en $backup_dir"
}

record() { mkdir -p "$state"; echo "$1" >> "$manifest"; }

install_file() { # modo origen destino
  install -Dm"$1" "$2" "$3"
  record "$3"
}

# Instala un .desktop con el Exec absoluto y «(vista previa)» en el nombre, para distinguirlo
# del de GNOME mientras conviven.
install_desktop() { # origen destino-nombre
  local dest="$apps_dir/$2" drop='^#nada#'
  mkdir -p "$apps_dir"
  # Los tipos MIME solo se conservan en Brújula (hacen falta para ser navegador); en el resto se
  # quitan para que la vista previa no se convierta en el gestor de carpetas por su cuenta.
  [[ $2 == "$brujula_desktop" ]] || drop='^MimeType='
  sed -E \
    -e "s|^Exec=([a-z-]+)|Exec=$bin/\1|" \
    -e 's|^(Name(\[[a-z_]+\])?)=(.*)$|\1=\3 (NDL)|' \
    -e "/$drop/d" \
    "$1" > "$dest"
  record "$dest"
}

do_install() {
  check
  backup
  mkdir -p "$state" "$bin"
  : > "$manifest"

  echo "== Monitor del sistema, Ajustes y Archivador =="
  local pkgs=(-p nodalix-monitor -p nodalix-settings) extra=()
  if [[ -d $root/apps/archivador ]]; then
    pkgs+=(-p nodalix-archivador)
    extra+=(archivador)
  fi
  (cd "$root/apps" && cargo build --release --locked "${pkgs[@]}")
  install_file 755 "$root/apps/target/release/nodalix-monitor" "$bin/nodalix-monitor"
  install_file 755 "$root/apps/target/release/nodalix-settings" "$bin/nodalix-settings"
  install_desktop "$root/apps/monitor/data/os.nodalix.Monitor.desktop" os.nodalix.Monitor.desktop
  install_desktop "$root/apps/ajustes/data/os.nodalix.Settings.desktop" os.nodalix.Settings.desktop
  if [[ ${extra[*]:-} == archivador ]]; then
    install_file 755 "$root/apps/target/release/nodalix-archivador" "$bin/nodalix-archivador"
    install_desktop "$root/apps/archivador/data/os.nodalix.Archivador.desktop" os.nodalix.Archivador.desktop
  fi

  echo "== Brújula =="
  if [[ ! -d $brujula_dir/.git ]]; then
    git clone "$brujula_url" "$brujula_dir"
  else
    git -C "$brujula_dir" fetch origin main
    git -C "$brujula_dir" checkout -q main
    git -C "$brujula_dir" merge --ff-only origin/main
  fi
  (cd "$brujula_dir" && cargo build --release --locked -p brujula)
  install_file 755 "$brujula_dir/target/release/brujula" "$bin/brujula"
  install_file 644 "$brujula_dir/packaging/icons/dev.danielmiguelt.Brujula.svg" \
    "$icons_dir/dev.danielmiguelt.Brujula.svg"
  install_desktop "$brujula_dir/packaging/$brujula_desktop" "$brujula_desktop"

  update-desktop-database "$apps_dir"
  gtk-update-icon-cache -qf "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" 2>/dev/null || true
  cat <<EOF

Listo. En el lanzador aparecen «Monitor del sistema (NDL)», «Ajustes (NDL)», «Brújula (NDL)» y,
si la rama lo trae, «Archivos (NDL)».
Los de GNOME siguen donde estaban. Para quitarlo todo: $0 uninstall
Brújula como predeterminado (opcional): $0 default-browser on
EOF
}

default_browser() {
  case "${1:-}" in
    on)  xdg-settings set default-web-browser "$brujula_desktop"
         xdg-settings get default-web-browser ;;
    off) local prev
         prev=$(cat "$backup_dir/navegador-predeterminado.txt" 2>/dev/null || true)
         [[ -n $prev ]] || prev=$(cat "$(ls -d "$HOME"/Respaldos/nodalix-pre-0.6.0-preview-*/ | tail -1)/navegador-predeterminado.txt" 2>/dev/null || true)
         [[ -n $prev ]] || die "no hay navegador anterior guardado; usa: xdg-settings set default-web-browser <su.desktop>"
         xdg-settings set default-web-browser "$prev"
         xdg-settings get default-web-browser ;;
    *)   die "uso: $0 default-browser on|off" ;;
  esac
}

uninstall() {
  [[ -f $manifest ]] || die "no hay nada instalado por este script ($manifest no existe)"
  if [[ $(xdg-settings get default-web-browser 2>/dev/null) == "$brujula_desktop" ]]; then
    default_browser off || echo "Restaura el navegador a mano con xdg-settings."
  fi
  while IFS= read -r f; do
    [[ -n $f && -e $f ]] && rm -f -- "$f"
  done < "$manifest"
  rm -f -- "$manifest"
  update-desktop-database "$apps_dir" 2>/dev/null || true
  echo "Quitado. Los datos de Brújula (~/.config/brujula, ~/.local/share/brujula, ~/.cache/brujula) se dejan; bórralos si quieres."
}

archivador() {
  need makepkg "sudo pacman -S base-devel"
  local out="$root/dist/native-archivador"
  python3 "$root/tools/build-archivador-package.py" --output "$out"
  (cd "$out" && ls -1 *.pkg.tar.zst 2>/dev/null || find . -name '*.pkg.tar.zst')
  cat <<EOF

Archivador sustituye a nautilus (conflicts/provides). Para probarlo:
  sudo pacman -U <el .pkg.tar.zst de arriba>     # pacman pregunta si reemplaza nautilus: responde que sí
  nautilus -q; nautilus
Vuelta atrás:
  sudo pacman -S nautilus                        # reemplaza nodalix-archivador por el de Arch
EOF
}

store() {
  need makepkg "sudo pacman -S base-devel"
  local work
  work=$(mktemp -d)
  cp -r "$root/packaging/nodalix-store/." "$work/"
  (cd "$work" && makepkg -sf --noconfirm)
  echo "Paquete en: $work"; ls "$work"/*.pkg.tar.zst
  cat <<EOF

Para instalarlo:   sudo pacman -U $work/nodalix-store-*.pkg.tar.zst
Vuelta atrás:      sudo pacman -U /var/cache/pacman/pkg/nodalix-store-0.5.0-*.pkg.tar.zst
                   (o el .pkg.tar.zst de la 0.5.2 que bajó el updater)
EOF
}

case "${1:-help}" in
  check) check ;;
  backup) backup ;;
  install) do_install ;;
  default-browser) default_browser "${2:-}" ;;
  uninstall) uninstall ;;
  archivador) archivador ;;
  store) store ;;
  *) sed -n '2,14p' "$0" ;;
esac
