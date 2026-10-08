#!/usr/bin/env bash
# A disposable Arch root; installs the complete candidate with real dependencies.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || exit 1
mkdir -p dist/migration-evidence
pacman -Syu --noconfirm
pacman -S --needed --noconfirm python libarchive glibc-locales
python - <<'PY'
from pathlib import Path
import subprocess,re
rows=[subprocess.check_output(['bsdtar','-xOf',str(p),'.PKGINFO'],text=True) for p in Path('dist/packages').glob('*.pkg.tar.zst')]
providers={re.split('[<>=]',line.split(' = ',1)[1])[0] for row in rows for line in row.splitlines() if line.startswith(('pkgname = ','provides = '))}
deps=sorted({line.split(' = ',1)[1] for row in rows for line in row.splitlines() if line.startswith('depend = ') and re.split('[<>=]',line.split(' = ',1)[1])[0] not in providers})
subprocess.run(['pacman','-S','--needed','--noconfirm',*deps],check=True)
PY
useradd -m migration-test
printf 'LANG=es_ES.UTF-8\n' > /etc/locale.conf
printf 'KEYMAP=es\n' > /etc/vconsole.conf
pacman -U --noconfirm --ask 4 dist/packages/*.pkg.tar.zst
nodalix-gnome-migrate --system
python - <<'PY'
import locale,json
from pathlib import Path
locale.setlocale(locale.LC_ALL,'es_ES.UTF-8')
assert Path('/usr/share/wayland-sessions/nodalix.desktop').is_file()
assert Path('/usr/share/gnome-background-properties/nodalix-wallpapers.xml').is_file()
assert Path('/etc/systemd/system/display-manager.service').readlink()==Path('/usr/lib/systemd/system/gdm.service')
assert Path('/usr/lib/libblur-effect-1.0.so').is_file()
assert not Path('/home/migration-test/.config/hypr').exists()
print('Clean package installation passed; ready for real GDM boot')
PY
