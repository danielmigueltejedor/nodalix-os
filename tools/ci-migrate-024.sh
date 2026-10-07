#!/usr/bin/env bash
# Real package transaction in a disposable container, no host services touched.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
mkdir -p dist/migration-evidence dist/baseline-024
pacman -Syu --noconfirm
pacman -S --needed --noconfirm python libarchive curl diffutils gnome-control-center greetd
python - <<'PY'
import hashlib,json,urllib.request
from pathlib import Path
root=Path('/src/dist/baseline-024')
base='https://github.com/danielmigueltejedor/nodalix-os/releases/download/0.2.4/'
with urllib.request.urlopen(base+'nodalix-manifest.json') as f:data=json.load(f)
for c in data['components']:
    p=root/c['asset']
    if not p.exists():urllib.request.urlretrieve(base+c['asset'],p)
    assert hashlib.file_digest(p.open('rb'),'sha256').hexdigest()==c['sha256'],p.name
(root/'manifest.json').write_text(json.dumps(data,indent=2))
PY
# Fixture installation needs --nodeps: this simulates the original installed
# package DB without fetching an obsolete Hyprland dependency set. Only the
# baseline uses these flags. The actual upgrade runs all dependencies/scripts.
pacman -Udd --noconfirm --noscriptlet dist/baseline-024/*.pkg.tar.zst
grep -qx 'VERSION_ID="0.2.4"' /etc/nodalix-release
id migration-test >/dev/null 2>&1 || useradd -m migration-test
mkdir -p /var/lib/AccountsService/users /home/migration-test/.config/hypr /home/migration-test/.config/localsend
printf '[User]\nSession=hyprland\nLanguage=es_ES.UTF-8\n' > /var/lib/AccountsService/users/migration-test
printf 'personal Hyprland configuration\n' > /home/migration-test/.config/hypr/hyprland.conf
printf 'personal identity fixture\n' > /home/migration-test/.config/localsend/identity.pem
systemctl enable greetd.service
cp /etc/greetd/config.toml dist/migration-evidence/greetd.before
sha256sum /home/migration-test/.config/{hypr/hyprland.conf,localsend/identity.pem} > dist/migration-evidence/personal.before
python /src/updater/nodalix-migrate-gnome --assets /src/dist/packages 2>&1 | tee dist/migration-evidence/upgrade.log
grep -qx 'VERSION_ID="0.3.0"' /etc/nodalix-release
[[ $(cat /run/nodalix-updater/reboot-required) == 0.3.0 ]]
[[ $(readlink /etc/systemd/system/display-manager.service) == /usr/lib/systemd/system/gdm.service ]]
pacman -Q nodalix-gnome nodalix-settings nodalix-control-center nodalix-integrations nodalix-video-wallpapers nodalix-ofono
! pacman -Qq | grep -Ex 'nodalix-shell|nodalix-greeter-theme'
pacman -Qo /usr/lib/systemd/user/nodalix-icloud-drive.service | grep nodalix-integrations
cmp dist/migration-evidence/greetd.before /var/lib/nodalix-updater/migrations/gnome-0.3.0/files/etc/greetd/config.toml
sha256sum -c dist/migration-evidence/personal.before
nodalix-updater status --json > dist/migration-evidence/status.json
python - <<'PY'
import json
from pathlib import Path
state=json.loads(Path('/src/dist/migration-evidence/status.json').read_text())
assert state['state']=='completed' and state['reboot_mandatory']
assert 'Session = nodalix' in Path('/var/lib/AccountsService/users/migration-test').read_text()
assert Path('/etc/systemd/system/sockets.target.wants/cups.socket').exists()
assert len(list(Path('/usr/share/backgrounds/nodalix/Animados').glob('*.mp4')))==16
print('0.2.4 → 0.3.0 verified; user files preserved; GDM and mandatory restart ready')
PY
if nodalix-updater update --assets /src/dist/packages --json > dist/migration-evidence/reboot-block.json; then
  echo 'Expected pending mandatory restart to block updates'; exit 1
fi
# Model next boot by removing only the /run marker; the persistent status must
# now report that the mandatory restart was fulfilled.
rm /run/nodalix-updater/reboot-required
nodalix-updater status --json | python -c 'import json,sys;assert not json.load(sys.stdin)["reboot_mandatory"]'
