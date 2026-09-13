#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)

if rg -n '/home/daniel|sudo -S|qs ipc|\.config/quickshell|\.local/state/quickshell' \
    "$repo_root/shell" "$repo_root/packaging"; then
    printf '%s\n' "Forbidden installed-system reference found" >&2
    exit 1
fi

rg -q 'ExecStart=/usr/bin/nodalix-shell' \
    "$repo_root/packaging/nodalix-shell/nodalix-shell.service"
rg -q 'qs --no-duplicate -c nodalix' \
    "$repo_root/packaging/nodalix-shell/nodalix-shell"

if rg -n 'hl\.bind\([^)]*SUPER \+ SPACE|ipc call launcher toggle' \
    "$repo_root/shell/hypr/quickshell.lua"; then
    printf '%s\n' "quickshell.lua must not hardcode BindingService-managed shortcuts" >&2
    exit 1
fi
rg -q 'k === "launcher" \? "SUPER \+ SPACE"' \
    "$repo_root/shell/services/BindingService.qml"
rg -q 'binds.generated.lua' "$repo_root/shell/hypr/quickshell.lua"

printf '%s\n' "PASS: installed-system static policy"
