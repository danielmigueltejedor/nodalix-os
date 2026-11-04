#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT
cc "$root/tests/native-restart.c" -o "$temp/native-restart" $(pkg-config --cflags --libs libadwaita-1 json-glib-1.0)
export NODALIX_TEST_ROOT="$root" NODALIX_TEST_BINARY="$temp/native-restart"
install -d -m700 "$temp/runtime" "$temp/config" "$temp/cache" "$temp/state"
# Xvfb sets DISPLAY, but a desktop Wayland socket would otherwise still win.
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DISPLAY -u XAUTHORITY \
  -u DBUS_STARTER_ADDRESS -u DBUS_STARTER_BUS_TYPE \
  XDG_RUNTIME_DIR="$temp/runtime" XDG_CONFIG_HOME="$temp/config" \
  XDG_CACHE_HOME="$temp/cache" XDG_STATE_HOME="$temp/state" \
  dbus-run-session -- bash -c '
  /usr/bin/python3 "$NODALIX_TEST_ROOT/tests/native-restart-service.py" &
  service=$!
  trap "kill $service" EXIT
  xvfb-run -a env GDK_BACKEND=x11 GTK_A11Y=none GIO_USE_VFS=local GTK_USE_PORTAL=0 GSK_RENDERER=cairo "$NODALIX_TEST_BINARY"
'
