#!/bin/sh
set -eu

if command -v yay >/dev/null 2>&1; then
    yay -Sua --noconfirm
fi
if command -v nodalix-apps >/dev/null 2>&1; then
    nodalix-apps update --all
fi

if command -v flatpak >/dev/null 2>&1; then
    flatpak update -y --noninteractive
fi
