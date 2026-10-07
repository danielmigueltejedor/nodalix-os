#!/usr/bin/env bash
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || exit 1
pacman -Syu --noconfirm
pacman -S --needed --noconfirm archiso python libarchive
python tools/build-iso.py --assets dist/packages --work dist/iso-installable --output dist/iso-output
sha256sum dist/iso-output/*.iso > dist/iso-output/SHA256SUMS
