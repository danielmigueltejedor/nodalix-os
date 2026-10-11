#!/usr/bin/python3
"""Boot the release ISO in QEMU, inject QA through its disposable serial console.

Uses the ISO's kernel, initramfs and real archiso root. Nothing is added to the
release image or installed on the host. Firmware boot-menu coverage is separate.
"""
import argparse
import base64
import os
from pathlib import Path
import selectors
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('iso', type=Path)
    parser.add_argument('--evidence', type=Path, default=ROOT/'dist/iso-evidence')
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    iso = args.iso.resolve()
    paths = subprocess.check_output(['bsdtar', '-tf', str(iso)], text=True).splitlines()
    kernel = next(p for p in paths if p.endswith('/vmlinuz-linux'))
    initrd = next(p for p in paths if p.endswith('/initramfs-linux.img'))
    for source, name in ((kernel, 'vmlinuz'), (initrd, 'initramfs.img')):
        with (args.evidence/name).open('wb') as output:
            subprocess.run(['bsdtar', '-xOf', str(iso), source], stdout=output, check=True)
    version = (ROOT/'VERSION').read_text().strip()
    label = 'NODALIX_'+version.replace('.', '').replace('-', '_').upper()
    accel = 'kvm' if os.access('/dev/kvm', os.R_OK | os.W_OK) else 'tcg'
    process = subprocess.Popen([
        'qemu-system-x86_64', '-accel', accel, '-cpu', 'max', '-m', '4096', '-smp', '2',
        '-kernel', str(args.evidence/'vmlinuz'), '-initrd', str(args.evidence/'initramfs.img'),
        '-append', f'archisobasedir=arch archisolabel={label} console=ttyS0,115200 audit=0 loglevel=3 systemd.firstboot=no',
        '-cdrom', str(iso), '-device', 'virtio-vga', '-display', 'none',
        '-serial', 'stdio', '-monitor', 'none', '-no-reboot',
    ], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic()+900
    transcript = b''; login_sent = shell_prepared = command_sent = passed = False
    try:
        with (args.evidence/'boot.log').open('wb') as log:
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    break
                for key, _ in selector.select(timeout=1):
                    data = os.read(key.fd, 65536)
                    if not data:
                        continue
                    log.write(data);log.flush();transcript += data
                    if not login_sent and b'login:' in transcript:
                        process.stdin.write(b'root\n');process.stdin.flush();login_sent = True
                    if login_sent and not shell_prepared and (b'# ' in transcript or b'#\x1b' in transcript):
                        process.stdin.write(b"stty -echo; printf '\\nNODALIX_QA_READY\\n'; exec /bin/bash --noprofile --norc -s\n")
                        process.stdin.flush();shell_prepared=True
                    if shell_prepared and not command_sent and (b'\nNODALIX_QA_READY\r\n' in transcript or b'\nNODALIX_QA_READY\n' in transcript):
                        code = base64.b64encode((ROOT/'tools/ci-guest-verify.py').read_bytes()).decode()
                        # Keep every serial line below the kernel's canonical
                        # input limit and avoid interactive zsh/readline redraws.
                        chunks='\n'.join(code[index:index+512] for index in range(0,len(code),512))
                        command = ("base64 -d > /run/nodalix-live-verify.py <<'NODALIX_QA_BASE64'\n"
                                   +chunks+"\nNODALIX_QA_BASE64\n"
                                   "NODALIX_CI_USER=nodalix NODALIX_CI_FRESH=1 NODALIX_CI_LIVE=1 python /run/nodalix-live-verify.py\n")
                        process.stdin.write(command.encode());process.stdin.flush();command_sent = True
                    if b'NODALIX_VM_MIGRATION_PASS\r\n' in transcript or b'NODALIX_VM_MIGRATION_PASS\n' in transcript:
                        passed = True
        if not passed:
            raise RuntimeError('ISO did not complete its real GDM/user-session checks; inspect boot.log')
        print('Release ISO kernel, archiso root, GDM and GNOME session passed in QEMU')
    finally:
        selector.close()
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill();process.wait()


if __name__ == '__main__':
    main()
