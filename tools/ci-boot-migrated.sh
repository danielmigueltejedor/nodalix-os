#!/usr/bin/env bash
# Boot the disposable migrated root in QEMU; never operate on the host desktop.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || exit 1
pacman -S --needed --noconfirm linux mkinitcpio qemu-system-x86 qemu-ui-opengl qemu-hw-display-virtio-gpu qemu-hw-display-virtio-vga e2fsprogs
# There are no further package transactions in this disposable container.
# Free the downloads before holding both the staged root and its disk image.
if [[ -f /.dockerenv || -f /run/.containerenv ]]; then
    find /var/cache/pacman/pkg -maxdepth 1 -type f -name '*.pkg.tar.*' -delete
fi
qa=${NODALIX_QA_VM_DIR:-/src/dist/vm-candidate}
[[ ! -e "$qa" ]] || { echo 'Choose a fresh disposable VM directory'; exit 1; }
mkdir -p -m 700 "$qa"
cat > "$qa/mkinitcpio.conf" <<'CONFIG'
MODULES=(virtio_pci virtio_blk virtio_net virtio_gpu ext4)
BINARIES=()
FILES=()
HOOKS=(base systemd modconf block filesystems)
CONFIG
kernel=$(find /usr/lib/modules -name pkgbase -exec dirname {} \; | head -1)
mkinitcpio -k "${kernel##*/}" -c "$qa/mkinitcpio.conf" -g "$qa/initramfs.img"
cp /boot/vmlinuz-linux "$qa/vmlinuz-linux"
guest="$qa/root"
mkdir -p "$guest"
for dir in usr etc var home root opt boot; do [[ ! -d /$dir ]] || cp -a /"$dir" "$guest/"; done
for dir in dev proc sys run tmp; do mkdir -p "$guest/$dir"; done
ln -s usr/bin "$guest/bin"; ln -s usr/bin "$guest/sbin"; ln -s usr/lib "$guest/lib"; ln -s usr/lib "$guest/lib64"
rm -rf "$guest/var/cache/pacman/pkg" "$guest/var/log/journal"
systemd-firstboot --root="$guest" --setup-machine-id --locale=es_ES.UTF-8 --timezone=UTC --hostname=nodalix-migration-test --force
cat > "$guest/etc/gdm/custom.conf" <<'GDM'
[daemon]
AutomaticLoginEnable=True
AutomaticLogin=migration-test
WaylandEnable=true
GDM
install -Dm755 /src/tools/ci-guest-verify.py "$guest/usr/local/bin/nodalix-guest-verify"
install -Dm755 /src/tools/ci-native-shell-cycle.py "$guest/usr/local/lib/nodalix/ci-native-shell-cycle.py"
mkdir -p "$guest/etc/systemd/sleep.conf.d"
printf '[Sleep]\nMemorySleepMode=deep\n' > "$guest/etc/systemd/sleep.conf.d/nodalix-qa.conf"
cat > "$guest/etc/systemd/system/nodalix-guest-verify.service" <<UNIT
[Unit]
Description=Verify disposable Nodalix migration boot

After=gdm.service
[Service]
Type=oneshot
Environment=NODALIX_CI_VERSION=$(tr -d '[:space:]' < /src/VERSION)
Environment=NODALIX_CI_FRESH=${NODALIX_CI_FRESH:-0}
Environment=NODALIX_CI_GUEST=1
ExecStart=/usr/local/bin/nodalix-guest-verify
StandardOutput=journal+console
StandardError=journal+console
[Install]
WantedBy=graphical.target
UNIT
# A user namespace used for local rehearsal cannot chown to guest UIDs. Fix
# only the disposable guest account at boot; Docker CI already has real owners.
mkdir -p "$guest/etc/systemd/system/gdm.service.d"
cat > "$guest/etc/systemd/system/gdm.service.d/qa-owner.conf" <<'OWNER'
[Service]
ExecStartPre=/usr/bin/chown -R migration-test:migration-test /home/migration-test
OWNER
# This helper exists only in the disposable guest, including while locked.
qa_extension="$guest/home/migration-test/.local/share/gnome-shell/extensions/nodalix-ci-native@example.invalid"
mkdir -p "$qa_extension"
cat > "$qa_extension/metadata.json" <<'QA'
{"uuid":"nodalix-ci-native@example.invalid","name":"Disposable native desktop test","description":"Guest-only test helper","shell-version":["51"],"session-modes":["user","unlock-dialog"]}
QA
cat > "$qa_extension/extension.js" <<'QA'
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
export default class QA extends Extension {
    enable() {
        this.previous = global.context.unsafe_mode;
        global.context.unsafe_mode = true;
    }
    disable() {
        global.context.unsafe_mode = this.previous;
    }
}
QA
mkdir -p "$guest/etc/systemd/system/graphical.target.wants"
ln -s ../nodalix-guest-verify.service "$guest/etc/systemd/system/graphical.target.wants/nodalix-guest-verify.service"
ln -sf /usr/lib/systemd/system/graphical.target "$guest/etc/systemd/system/default.target"
truncate -s 14G "$qa/migrated.raw"
mkfs.ext4 -q -F -d "$guest" "$qa/migrated.raw"
(cd "$qa" && exec python /src/tools/ci-wake-guest.py) > /src/dist/migration-evidence/suspend.log 2>&1 &
wake_pid=$!
trap 'if kill -0 "$wake_pid" 2>/dev/null; then kill "$wake_pid"; fi' EXIT
timeout 900 qemu-system-x86_64 -accel tcg -cpu max -m 4096 -smp 2 \
  -kernel "$qa/vmlinuz-linux" -initrd "$qa/initramfs.img" \
  -append 'root=/dev/vda rw console=ttyS0 audit=0 loglevel=3 systemd.firstboot=no' \
  -drive "file=$qa/migrated.raw,format=raw,if=virtio" -device virtio-vga \
  -qmp "unix:$qa/qmp.sock,server=on,wait=off" \
  -display none -serial "file:/src/dist/migration-evidence/boot.log" -monitor none -no-reboot
wait "$wake_pid"
cat /src/dist/migration-evidence/suspend.log
cat /src/dist/migration-evidence/boot.log
grep -q 'NODALIX_VM_MIGRATION_PASS' /src/dist/migration-evidence/boot.log
