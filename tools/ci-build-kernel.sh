#!/usr/bin/env bash
# Build only inside the disposable Arch Linux CI container.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
pacman -Syu --needed --noconfirm base-devel git bc clang cpio libelf lld llvm openssl pahole \
  patch perl rust rust-bindgen rust-src zstd
useradd -m builder
install -d -o builder -g builder /src/dist/kernel /tmp/nodalix-kernel
jobs=$(nproc)
runuser -u builder -- env MAKEFLAGS="-j$jobs" \
  bash /src/tools/build-nodalix-kernel.sh /tmp/nodalix-kernel --cleanbuild
install -m644 /tmp/nodalix-kernel/linux-nodalix-*.pkg.tar.zst /src/dist/kernel/
(cd /src/dist/kernel && sha256sum linux-nodalix-*.pkg.tar.zst > SHA256SUMS)
