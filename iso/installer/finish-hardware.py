#!/usr/bin/env python3
"""Finalize the newly installed target; never run by the ISO builder."""
import json
import subprocess
from pathlib import Path


def hardware_policy(profile, virtual):
    optimized = not virtual and profile['tier'] in {'znver4', 'x86-64-v4', 'x86-64-v3'}
    return {'optimized': optimized, 'kernel': profile['kernel'] if optimized else 'linux',
            'reason': 'virtual machine' if virtual else profile['reason']}


def main():
    profile = json.loads(subprocess.check_output(['nodalix-hardware-profile', 'detect', '--json'], text=True))
    virtual = subprocess.run(['systemd-detect-virt', '--vm', '--quiet']).returncode == 0
    policy = hardware_policy(profile, virtual)
    report = {'detected': profile, 'selection': policy, 'optimization_applied': False}
    # Ensure the recovery kernel really exists, irrespective of menu choices.
    subprocess.run(['pacman', '-S', '--needed', '--noconfirm', 'linux', 'linux-headers'], check=True)
    if policy['optimized']:
        result = subprocess.run(['nodalix-hardware-profile', 'apply'])
        report['optimization_applied'] = result.returncode == 0
        if result.returncode:
            report['warning'] = 'Optimized kernel unavailable. The Arch recovery kernel remains installed.'
            print(report['warning'])
    if Path('/boot/grub/grub.cfg').exists():
        subprocess.run(['grub-mkconfig', '-o', '/boot/grub/grub.cfg'], check=True)
    subprocess.run(['mkinitcpio', '-P'], check=True)
    destination = Path('/var/log/nodalix-installer/hardware.json')
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
