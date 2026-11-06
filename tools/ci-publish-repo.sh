#!/usr/bin/env bash
# Build the signed [nodalix] repository for GitHub Pages from a directory of packages.
# Run only inside the disposable Arch CI container of the publish workflow.
# Inputs (environment): NODALIX_REPO_GPG_KEY (armored private key), optionally
# NODALIX_REPO_GPG_PASSPHRASE. Layout written: dist/pages/x86_64/{nodalix.db,*.pkg.tar.zst,...}
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
: "${NODALIX_REPO_GPG_KEY:?Missing the NODALIX_REPO_GPG_KEY secret}"

expected=3DE029728CDE1828E18B437930D356AC0F6AB1CF
# GitHub Pages sites are limited to 1 GB; stay well below it.
max_bytes=$((900 * 1024 * 1024))

packages=${1:-dist/packages}
site=${2:-dist/pages}

pacman -Syu --noconfirm
pacman -S --needed --noconfirm gnupg

export GNUPGHOME
GNUPGHOME=$(mktemp -d)
chmod 700 "$GNUPGHOME"
trap 'gpgconf --kill gpg-agent 2>/dev/null || true; rm -rf -- "$GNUPGHOME"' EXIT

if [[ -n ${NODALIX_REPO_GPG_PASSPHRASE:-} ]]; then
  # repo-add and gpg run non-interactively, so feed the passphrase through loopback pinentry.
  printf '%s' "$NODALIX_REPO_GPG_PASSPHRASE" > "$GNUPGHOME/passphrase"
  chmod 600 "$GNUPGHOME/passphrase"
  printf 'allow-loopback-pinentry\n' > "$GNUPGHOME/gpg-agent.conf"
  printf 'pinentry-mode loopback\npassphrase-file %s\n' "$GNUPGHOME/passphrase" > "$GNUPGHOME/gpg.conf"
fi

printf '%s\n' "$NODALIX_REPO_GPG_KEY" | gpg --batch --import
fingerprint=$(gpg --batch --list-secret-keys --with-colons | awk -F: '$1 == "fpr" {print $10; exit}')
if [[ $fingerprint != "$expected" ]]; then
  echo "The signing key $fingerprint is not the Nodalix repository key" >&2
  exit 1
fi

size=$(du -sb -- "$packages" | cut -f1)
if (( size > max_bytes )); then
  echo "Packages take $size bytes, over the GitHub Pages budget of $max_bytes" >&2
  exit 1
fi

rm -rf -- "$site"
bash tools/build-pacman-repo.sh "$packages" "$site/x86_64" "$fingerprint"
cat > "$site/index.html" <<'HTML'
<!doctype html>
<meta charset="utf-8">
<title>Nodalix OS package repository</title>
<p>This is the signed pacman repository of Nodalix OS. It is meant for pacman, not for browsers:
<code>Server = https://danielmigueltejedor.github.io/nodalix-os/$arch</code></p>
HTML
touch "$site/.nojekyll"
echo "Repository ready in $site ($(du -sh -- "$site" | cut -f1))"
