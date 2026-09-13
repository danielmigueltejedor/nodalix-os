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

printf '%s\n' "PASS: installed-system static policy"
