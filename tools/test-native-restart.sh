#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT
cc "$root/tests/native-restart.c" -o "$temp/native-restart" $(pkg-config --cflags --libs libadwaita-1 json-glib-1.0)
export NODALIX_TEST_ROOT="$root" NODALIX_TEST_BINARY="$temp/native-restart"
dbus-run-session -- bash -c '
  /usr/bin/python3 "$NODALIX_TEST_ROOT/tests/native-restart-service.py" &
  service=$!
  trap "kill $service" EXIT
  xvfb-run -a env GTK_A11Y=none GIO_USE_VFS=local GTK_USE_PORTAL=0 GSK_RENDERER=cairo "$NODALIX_TEST_BINARY"
'
