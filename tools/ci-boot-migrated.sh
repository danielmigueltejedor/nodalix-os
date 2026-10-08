#!/usr/bin/env bash
# Boot the disposable migrated root in QEMU; never operate on the host desktop.
set -euo pipefail
[[ ${NODALIX_CI_CONTAINER:-} == 1 ]] || exit 1
pacman -S --needed --noconfirm linux mkinitcpio qemu-system-x86 qemu-ui-opengl qemu-hw-display-virtio-gpu qemu-hw-display-virtio-vga e2fsprogs
mkdir -p /src/dist/vm-candidate
qa=/src/dist/vm-candidate
cat > "$qa/mkinitcpio.conf" <<'CONFIG'
MODULES=(virtio_pci virtio_blk virtio_net virtio_gpu ext4)
BINARIES=()
FILES=()
HOOKS=(base systemd autodetect modconf block filesystems)
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
systemd-firstboot --root="$guest" --setup-machine-id --locale=C.UTF-8 --timezone=UTC --hostname=nodalix-migration-test --force
cat > "$guest/etc/gdm/custom.conf" <<'GDM'
[daemon]
AutomaticLoginEnable=True
AutomaticLogin=migration-test
WaylandEnable=true
GDM
install -Dm755 /src/tools/ci-guest-verify.py "$guest/usr/local/bin/nodalix-guest-verify"
cat > "$guest/etc/systemd/system/nodalix-guest-verify.service" <<UNIT
[Unit]
Description=Verify disposable Nodalix migration boot

After=gdm.service
[Service]
Type=oneshot
Environment=NODALIX_CI_FRESH=${NODALIX_CI_FRESH:-0}
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
mkdir -p "$guest/etc/systemd/system/graphical.target.wants"
ln -s ../nodalix-guest-verify.service "$guest/etc/systemd/system/graphical.target.wants/nodalix-guest-verify.service"
ln -sf /usr/lib/systemd/system/graphical.target "$guest/etc/systemd/system/default.target"
truncate -s 14G "$qa/migrated.raw"
mkfs.ext4 -q -F -d "$guest" "$qa/migrated.raw"
timeout 600 qemu-system-x86_64 -accel tcg -cpu max -m 4096 -smp 2 \
  -kernel "$qa/vmlinuz-linux" -initrd "$qa/initramfs.img" \
  -append 'root=/dev/vda rw console=ttyS0 audit=0 loglevel=3 systemd.firstboot=no' \
  -drive "file=$qa/migrated.raw,format=raw,if=virtio" -device virtio-vga \
  -display none -serial "file:/src/dist/migration-evidence/boot.log" -monitor none -no-reboot
cat /src/dist/migration-evidence/boot.log
grep -q '^NODALIX_VM_MIGRATION_PASS' /src/dist/migration-evidence/boot.log
