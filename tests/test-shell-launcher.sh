#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

mkdir -p "$tmp/bin" "$tmp/home/.local/state/quickshell"
printf '%s' old > "$tmp/home/.local/state/quickshell/settings.json"
printf '%s' legacy > "$tmp/home/.local/state/quickshell/pinned.json"
mkdir -p "$tmp/home/.local/state/nodalix"
printf '%s' partial > "$tmp/home/.local/state/nodalix/settings.json"
printf '%s\n' '#!/bin/sh' 'printf "%s\\n" "$*" > "$QS_ARGS"' > "$tmp/bin/qs"
chmod +x "$tmp/bin/qs"

HOME="$tmp/home" PATH="$tmp/bin:$PATH" QS_ARGS="$tmp/args" \
    "$repo_root/packaging/nodalix-shell/nodalix-shell"

test "$(cat "$tmp/home/.local/state/nodalix/settings.json")" = old
test "$(cat "$tmp/home/.local/state/nodalix/pinned.json")" = legacy
test -e "$tmp/home/.local/state/nodalix/.migrated-from-quickshell-v1"
test "$(cat "$tmp/args")" = "--no-duplicate -c nodalix"

printf '%s' current > "$tmp/home/.local/state/nodalix/settings.json"
HOME="$tmp/home" PATH="$tmp/bin:$PATH" QS_ARGS="$tmp/args" \
    "$repo_root/packaging/nodalix-shell/nodalix-shell"
test "$(cat "$tmp/home/.local/state/nodalix/settings.json")" = current

printf '%s\n' "PASS: shell launcher migration is idempotent and selects nodalix"
