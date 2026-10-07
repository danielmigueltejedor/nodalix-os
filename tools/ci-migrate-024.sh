#!/usr/bin/env bash
# Real package transaction in a disposable container, no host services touched.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || { echo 'Disposable CI container required'; exit 1; }
mkdir -p dist/migration-evidence dist/baseline-024
pacman -Syu --noconfirm
pacman -S --needed --noconfirm python libarchive curl diffutils
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
# Resolve the original package dependencies in the clean Arch root, then install
# the unmodified official 0.2.4 packages with dependency checks and scriptlets.
python - <<'DEPS'
import subprocess,re
from pathlib import Path
rows=[subprocess.check_output(['bsdtar','-xOf',str(p),'.PKGINFO'],text=True) for p in Path('dist/baseline-024').glob('*.pkg.tar.zst')]
provided={line.split(' = ',1)[1].split('=')[0] for row in rows for line in row.splitlines() if line.startswith(('pkgname = ','provides = '))}
deps=sorted({line.split(' = ',1)[1] for row in rows for line in row.splitlines() if line.startswith('depend = ') and re.split('[<>=]',line.split(' = ',1)[1])[0] not in provided})
subprocess.run(['pacman','-S','--needed','--noconfirm',*deps,'xdg-desktop-portal-hyprland'],check=True)
DEPS
pacman -U --noconfirm dist/baseline-024/*.pkg.tar.zst
grep -qx 'VERSION_ID="0.2.4"' /etc/nodalix-release
id migration-test >/dev/null 2>&1 || useradd -m migration-test
mkdir -p /var/lib/AccountsService/users /home/migration-test/.config/hypr /home/migration-test/.config/localsend
localedef -i es_ES -f UTF-8 es_ES.UTF-8
printf '[User]\nSession=hyprland\nLanguage=es_ES.UTF-8\n' > /var/lib/AccountsService/users/migration-test
printf 'personal Hyprland configuration\n' > /home/migration-test/.config/hypr/hyprland.conf
printf 'personal identity fixture\n' > /home/migration-test/.config/localsend/identity.pem
mkdir -p /home/migration-test/.local/state/nodalix
printf '{"wallpaper":{"path":"/usr/share/backgrounds/nodalix/animated/nodalix-aurora-forest-4k.mp4","type":"video"},"bar":{"clock":{"use24h":true}}}\n' > /home/migration-test/.local/state/nodalix/settings.json
printf '{"pinned":["org.gnome.Nautilus"]}\n' > /home/migration-test/.local/state/nodalix/pinned.json
chown -R migration-test:migration-test /home/migration-test
systemctl disable gdm.service >/dev/null 2>&1 || true
systemctl enable greetd.service
cp /etc/greetd/config.toml dist/migration-evidence/greetd.before
sha256sum /home/migration-test/.config/{hypr/hyprland.conf,localsend/identity.pem} > dist/migration-evidence/personal.before
python /src/updater/nodalix-migrate-gnome --assets /src/dist/packages 2>&1 | tee dist/migration-evidence/upgrade.log
grep -qx 'VERSION_ID="0.3.0"' /etc/nodalix-release
[[ $(cat /run/nodalix-updater/reboot-required) == 0.3.0 ]]
[[ $(readlink /etc/systemd/system/display-manager.service) == /usr/lib/systemd/system/gdm.service ]]
pacman -Q nodalix-gnome nodalix-settings nodalix-control-center nodalix-integrations nodalix-video-wallpapers nodalix-ofono
! pacman -Qq | grep -E '^(hypr|quickshell|greetd|nodalix-shell$|nodalix-greeter-theme$|nodalix-hymission$|nodalix-wallpaper-engine$|xdg-desktop-portal-hyprland$)'
! test -e /usr/share/wayland-sessions/hyprland.desktop
! test -e /etc/xdg/quickshell
! test -e /etc/greetd
pacman -Qo /usr/lib/systemd/user/nodalix-icloud-drive.service | grep nodalix-integrations
cmp dist/migration-evidence/greetd.before /var/lib/nodalix-updater/migrations/gnome-0.3.0/files/etc/greetd/config.toml
sha256sum /var/lib/nodalix-updater/migrations/gnome-0.3.0/files/home/migration-test/.config/hypr/hyprland.conf > dist/migration-evidence/hyprland.backup
[[ $(cut -d' ' -f1 dist/migration-evidence/hyprland.backup) == $(head -1 dist/migration-evidence/personal.before | cut -d' ' -f1) ]]
tail -1 dist/migration-evidence/personal.before | sha256sum -c
! test -e /home/migration-test/.config/hypr
nodalix-updater status --json > dist/migration-evidence/status.json
python - <<'PY'
import json
from pathlib import Path
state=json.loads(Path('/src/dist/migration-evidence/status.json').read_text())
assert state['state']=='completed' and state['reboot_mandatory']
assert 'Session = nodalix' in Path('/var/lib/AccountsService/users/migration-test').read_text()
assert Path('/etc/systemd/system/sockets.target.wants/cups.socket').exists()
assert len(list(Path('/usr/share/backgrounds/nodalix/Animados').glob('*.mp4')))==16
profile=json.loads(Path('/usr/share/nodalix/gnome-extensions.json').read_text())
for uuid in profile:assert Path('/usr/share/gnome-shell/extensions',uuid,'extension.js').is_file(),uuid
print('0.2.4 → 0.3.0 verified; user files preserved; GDM and mandatory restart ready')
PY
if nodalix-updater update --assets /src/dist/packages --json > dist/migration-evidence/reboot-block.json; then
  echo 'Expected pending mandatory restart to block updates'; exit 1
fi
# Model next boot by removing only the /run marker; the persistent status must
# now report that the mandatory restart was fulfilled.
rm /run/nodalix-updater/reboot-required
nodalix-updater status --json | python -c 'import json,sys;assert not json.load(sys.stdin)["reboot_mandatory"]'
