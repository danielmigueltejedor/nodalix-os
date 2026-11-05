#!/usr/bin/env bash
# Preserve the explicit preview build entry point.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
exec bash "$root/tools/build-nodalix-shell.sh" --preview "$@"
