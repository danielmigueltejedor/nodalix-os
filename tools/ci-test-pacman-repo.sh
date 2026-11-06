#!/usr/bin/env bash
# Prove the signed repository works with real pacman, using a throwaway key.
# Run only inside the disposable Arch CI container.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
pacman -Syu --noconfirm
pacman -S --needed --noconfirm gnupg

export GNUPGHOME
GNUPGHOME=$(mktemp -d)
chmod 700 "$GNUPGHOME"
gpg --batch --pinentry-mode loopback --passphrase '' \
  --quick-generate-key 'Nodalix CI repository <ci@nodalix.invalid>' ed25519 sign never
key=$(gpg --batch --list-secret-keys --with-colons | awk -F: '$1 == "fpr" {print $10; exit}')

repo=$(mktemp -d)
# pacman downloads as the unprivileged alpm user, which must be able to read the repository.
chmod 755 "$repo"
bash tools/build-pacman-repo.sh dist/packages "$repo/nodalix" "$key"
for name in nodalix.db nodalix.db.sig nodalix.files; do
  test -f "$repo/nodalix/$name" && ! test -L "$repo/nodalix/$name"
done

chmod -R a+rX "$repo"
mkdir -p "$repo/cache"
chmod 777 "$repo/cache"
# A client trusts only the repository key it was given.
gpg --batch --export "$key" > "$repo/nodalix.gpg"
pacman-key --init
pacman-key --add "$repo/nodalix.gpg"
pacman-key --lsign-key "$key"
cat >> /etc/pacman.conf <<CONF

[nodalix]
SigLevel = Required DatabaseRequired TrustedOnly
Server = file://$repo/nodalix
CONF
pacman -Sy
pacman -Si nodalix/nodalix-store >/dev/null
# The package signature is checked on download (dependencies are not needed for that).
pacman -Sddw --noconfirm --cachedir "$repo/cache" nodalix/nodalix-store
test -f "$repo"/cache/nodalix-store-*.pkg.tar.zst

# A tampered database must be rejected.
printf 'tampered' >> "$repo/nodalix/nodalix.db"
if output=$(pacman -Syy 2>&1); then
  echo 'pacman accepted a database that no longer matches its signature' >&2
  exit 1
fi
printf '%s\n' "$output"
grep -Eiq 'nodalix.*(signature|PGP)' <<<"$output" || {
  echo 'The rejection was not caused by the signature check' >&2
  exit 1
}
echo 'Signed repository verified with real pacman'
