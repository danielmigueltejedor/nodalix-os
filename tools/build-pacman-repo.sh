#!/usr/bin/env bash
# Build the signed [nodalix] pacman repository from a directory of packages.
# Usage: build-pacman-repo.sh PACKAGES_DIR OUTPUT_DIR KEY_ID
# The private key must already be in the GnuPG home that GNUPGHOME points to.
# Needs repo-add (pacman) and gpg; runs in an Arch container or VM.
set -euo pipefail
packages=${1:?directory with the built *.pkg.tar.zst files}
output=${2:?empty output directory}
key=${3:?GnuPG key id or fingerprint that signs packages and database}

if [[ -e $output && -n $(ls -A -- "$output") ]]; then
  echo "Output directory $output must be empty" >&2
  exit 1
fi
mkdir -p -- "$output"

shopt -s nullglob
files=()
for file in "$packages"/*.pkg.tar.zst; do
  [[ $file == *-debug-*.pkg.tar.zst ]] && continue
  files+=("$file")
done
[[ ${#files[@]} -gt 0 ]] || { echo "No packages found in $packages" >&2; exit 1; }

for file in "${files[@]}"; do
  cp -- "$file" "$output/"
  gpg --batch --yes --no-armor --local-user "$key" --detach-sign -- "$output/${file##*/}"
  gpg --batch --verify -- "$output/${file##*/}.sig" "$output/${file##*/}" 2>/dev/null
done

(
  cd -- "$output"
  repo-add --sign --key "$key" --verify nodalix.db.tar.gz ./*.pkg.tar.zst
  # repo-add leaves nodalix.db and nodalix.files as symlinks; static hosts need real files.
  for name in db files; do
    cp --remove-destination -- "nodalix.$name.tar.gz" "nodalix.$name"
    cp --remove-destination -- "nodalix.$name.tar.gz.sig" "nodalix.$name.sig"
  done
)
echo "Signed repository with ${#files[@]} packages in $output"
