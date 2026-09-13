#!/usr/bin/env bash
set -euo pipefail

repo="${NODALIX_REPOSITORY:-danielmigueltejedor/nodalix-os}"
channel="${NODALIX_CHANNEL:-beta}"

say() { printf '\033[1;36m::\033[0m %s\n' "$*"; }
die() { printf '\033[1;31mError:\033[0m %s\n' "$*" >&2; exit 1; }

[[ -e /etc/arch-release ]] || die "Este instalador requiere Arch Linux."
[[ "$channel" == "stable" || "$channel" == "beta" ]] || die "NODALIX_CHANNEL debe ser stable o beta."
command -v curl >/dev/null || die "Falta curl."
command -v python3 >/dev/null || die "Falta Python."
command -v pacman >/dev/null || die "Falta pacman."

if [[ $EUID -eq 0 ]]; then
    as_root=()
elif command -v sudo >/dev/null; then
    as_root=(sudo)
else
    die "Ejecuta el instalador como root o instala sudo."
fi

work=$(mktemp -d -t nodalix-install.XXXXXX)
trap 'rm -rf -- "$work"' EXIT

say "Buscando la versión más reciente de Nodalix ($channel)…"
curl -fsSL -H 'Accept: application/vnd.github+json' \
    "https://api.github.com/repos/$repo/releases?per_page=30" \
    -o "$work/releases.json"

selection=$(python3 - "$channel" "$work/releases.json" <<'PY'
import json, pathlib, sys

channel = sys.argv[1]
releases = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
for release in releases:
    if release.get("draft") or (channel == "stable" and release.get("prerelease")):
        continue
    manifest = next((asset for asset in release.get("assets", []) if asset.get("name") == "nodalix-manifest.json"), None)
    if manifest:
        print(f"{release['tag_name']}\t{manifest['browser_download_url']}")
        break
else:
    raise SystemExit("No compatible Nodalix release with a manifest was found")
PY
) || die "No se encontró una versión compatible."

release_tag=${selection%%$'\t'*}
manifest_url=${selection#*$'\t'}
say "Descargando Nodalix $release_tag…"
curl -fsSL "$manifest_url" -o "$work/nodalix-manifest.json"

python3 - "$work/nodalix-manifest.json" "$work/assets.tsv" <<'PY'
import json, pathlib, platform, re, sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if manifest.get("schema") != 1 or manifest.get("schema_version") != 1:
    raise SystemExit("Unsupported Nodalix manifest schema")
if manifest.get("arch", "x86_64") not in ("any", platform.machine()):
    raise SystemExit("The release architecture does not match this computer")
lines = []
for item in manifest.get("components", []):
    if not item.get("required"):
        continue
    asset = str(item.get("asset", ""))
    digest = str(item.get("sha256", "")).lower()
    if not asset or "/" in asset or "\\" in asset or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise SystemExit("Unsafe component metadata in release manifest")
    lines.append(f"{asset}\t{digest}")
if not lines:
    raise SystemExit("The release has no required components")
pathlib.Path(sys.argv[2]).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

packages=()
while IFS=$'\t' read -r asset digest; do
    destination="$work/$asset"
    curl -fL "https://github.com/$repo/releases/download/$release_tag/$asset" -o "$destination"
    printf '%s  %s\n' "$digest" "$destination" | sha256sum --check --status \
        || die "La verificación de $asset ha fallado."
    packages+=("$destination")
done < "$work/assets.tsv"

say "Instalando dependencias oficiales…"
"${as_root[@]}" pacman -S --needed --noconfirm \
    quickshell qt6-declarative python python-dbus python-gobject python-typer \
    bluez bluez-utils bluez-obex gtk4 libadwaita polkit minisign

say "Instalando Nodalix $release_tag…"
"${as_root[@]}" pacman -U --needed --noconfirm "${packages[@]}"
"${as_root[@]}" systemctl enable --now nodalix-update-check.timer
systemctl --user daemon-reload || true
systemctl --user enable --now nodalix-shell.service || true

say "Nodalix $release_tag está instalado. Cierra sesión y vuelve a entrar para completar la integración."
