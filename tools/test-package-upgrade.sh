#!/usr/bin/env bash
set -euo pipefail

BASELINE_DIR=${1:?baseline directory required}
CANDIDATE_DIR=${2:?candidate directory required}
TARGET_VERSION=${3:?target version required}
SCENARIO=${4:-normal}

ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

mkdir -p \
  "$ROOT/var/lib/pacman/local" \
  "$ROOT/var/cache/pacman/pkg"

shopt -s nullglob
baseline=("$BASELINE_DIR"/*.pkg.tar.zst)
candidate=("$CANDIDATE_DIR"/*.pkg.tar.zst)

if (( ${#candidate[@]} == 0 )); then
    echo "No candidate packages found" >&2
    exit 2
fi

pacman_root() {
    pacman \
        --root "$ROOT" \
        --dbpath "$ROOT/var/lib/pacman" \
        --cachedir "$ROOT/var/cache/pacman/pkg" \
        --logfile "$ROOT/pacman.log" \
        "$@"
}

install_packages() {
    pacman_root \
        -Udd \
        --noconfirm \
        --needed \
        --noscriptlet \
        "$@"
}

package_name() {
    bsdtar -xOf "$1" .PKGINFO \
        | sed -n 's/^pkgname = //p' \
        | head -n1
}

package_version() {
    bsdtar -xOf "$1" .PKGINFO \
        | sed -n 's/^pkgver = //p' \
        | head -n1
}

echo "=== Scenario: $SCENARIO ==="
echo "=== Target: $TARGET_VERSION ==="

if (( ${#baseline[@]} > 0 )); then
    echo "=== Installing baseline ==="
    install_packages "${baseline[@]}"
fi

if [[ "$SCENARIO" == "legacy-cursor" ]]; then
    echo "=== Recreating known legacy cursor state ==="

    if pacman_root -Q nodalix-cursor-theme >/dev/null 2>&1; then
        echo "Legacy cursor scenario requires a baseline without nodalix-cursor-theme" >&2
        exit 1
    fi

    mkdir -p "$ROOT/usr/share/icons/Nodalix/cursors"

    printf '[Icon Theme]\nName=Nodalix legacy\n' \
        > "$ROOT/usr/share/icons/Nodalix/index.theme"

    printf '[Icon Theme]\nInherits=Adwaita\n' \
        > "$ROOT/usr/share/icons/Nodalix/cursor.theme"

    printf 'legacy-cursor\n' \
        > "$ROOT/usr/share/icons/Nodalix/cursors/left_ptr"

    if pacman_root -Qo /usr/share/icons/Nodalix/index.theme >/dev/null 2>&1; then
        echo "Legacy cursor fixture unexpectedly belongs to a package" >&2
        exit 1
    fi
fi

echo "=== Installing candidate ==="
install_packages "${candidate[@]}"

echo "=== Running packaged migrations ==="

migrator="$ROOT/usr/lib/nodalix/migrate-cursor-theme"

if [[ -x "$migrator" ]]; then
    NODALIX_ROOT="$ROOT" "$migrator"
fi

cursor_payload="$ROOT/usr/share/nodalix/cursor-theme/Nodalix"
cursor_link="$ROOT/usr/share/icons/Nodalix"

if [[ -d "$cursor_payload/cursors" ]]; then
    if [[ ! -L "$cursor_link" ]]; then
        echo "Nodalix cursor compatibility link was not created" >&2
        exit 1
    fi

    link_target=$(readlink "$cursor_link")

    if [[ "$link_target" != "/usr/share/nodalix/cursor-theme/Nodalix" ]]; then
        echo "Unexpected cursor link target: $link_target" >&2
        exit 1
    fi
fi

if [[ "$SCENARIO" == "legacy-cursor" ]]; then
    migration_dir="$ROOT/var/lib/nodalix-updater/migrations/cursor-theme-layout-v1"

    if ! compgen -G "$migration_dir/legacy-Nodalix-*" >/dev/null; then
        echo "Legacy cursor backup was not created" >&2
        exit 1
    fi
fi

echo "=== Validating installed package versions ==="

for pkg in "${candidate[@]}"; do
    name=$(package_name "$pkg")
    expected=$(package_version "$pkg")
    installed=$(pacman_root -Q "$name" | awk '{print $2}')

    if [[ "$installed" != "$expected" ]]; then
        echo "$name: expected $expected, got $installed" >&2
        exit 1
    fi

    pacman_root -Qk "$name"
done

echo "=== Validating Nodalix identity ==="

release_file="$ROOT/etc/nodalix-release"

if [[ ! -f "$release_file" ]]; then
    echo "Missing /etc/nodalix-release" >&2
    exit 1
fi

actual_version=$(sed -n 's/^VERSION_ID="\?\([^" ]*\)"\?$/\1/p' "$release_file")

if [[ "$actual_version" != "$TARGET_VERSION" ]]; then
    echo "Expected VERSION_ID=$TARGET_VERSION, got $actual_version" >&2
    exit 1
fi

echo "=== Validating ownership of candidate payload ==="

for pkg in "${candidate[@]}"; do
    name=$(package_name "$pkg")

    while IFS= read -r rel; do
        rel=${rel#./}

        case "$rel" in
            ""|.BUILDINFO|.INSTALL|.MTREE|.PKGINFO|*/) continue;;
        esac

        path="/$rel"
        owner=$(pacman_root -Qoq "$ROOT/$rel" 2>/dev/null || true)

        if [[ "$owner" != "$name" ]]; then
            echo "Ownership mismatch: $path expected $name, got ${owner:-UNOWNED}" >&2
            exit 1
        fi
    done < <(bsdtar -tf "$pkg")
done

echo "=== Upgrade test passed ==="
cat "$release_file"
