#!/usr/bin/python3
"""Print an opt-in keyd config for physical keyboard interfaces only."""
from pathlib import Path
import subprocess


def bitmap(path):
    # sysfs prints most-significant machine words first.
    return int(''.join(word.zfill(16) for word in path.read_text().split()), 16)


def keyboard_ids(root=Path('/sys/class/input')):
    result=[]
    for event in sorted(root.glob('event*')):
        device=event/'device'
        try:
            if '/devices/virtual/' in str(device.resolve()):
                continue
            keys=bitmap(device/'capabilities/key')
            rel=bitmap(device/'capabilities/rel');absolute=bitmap(device/'capabilities/abs')
            # Full typing keyboard, no relative/absolute axes or mouse buttons.
            if rel & 0x103 or absolute & 0x3 or any(keys & (1<<key) for key in range(0x110,0x118)):
                continue
            if not all(keys & (1<<key) for key in (16,17,18,19,20,21,22,23,24,25,28,30,44,46,47,57,125)):
                continue
            vendor=int((device/'id/vendor').read_text(),16)
            product=int((device/'id/product').read_text(),16)
            if vendor==0x0fac:
                continue
            name=(device/'name').read_text().rstrip('\n').encode()
            # keyd 2.6.0 truncates long names to 64 bytes; its ID includes the
            # adjacent event path when the name has no terminating NUL.
            if len(name)>=64:
                name=name[:64]+('/dev/input/'+event.name).encode()
            count=keys.bit_count()
            value=5183
            for byte in count.to_bytes(4,'big')+bytes([absolute&255,rel&255])+name:
                value=(value*33+byte)&0xffffffff
            result.append(f'{vendor:04x}:{product:04x}:{value:08x}')
        except (OSError,ValueError):
            continue
    return sorted(set(result))


if __name__=='__main__':
    version=subprocess.check_output(['keyd','-v'],text=True).strip()
    if not version.startswith('keyd v2.6.0 '):
        raise SystemExit('Fingerprint generation is validated for keyd 2.6.0; review before using another version')
    ids=keyboard_ids()
    if not ids:
        raise SystemExit('No suitable physical typing keyboard found; no wildcard config generated')
    print('[ids]\n'+'\n'.join(ids)+'\n\n[meta]\nc = C-c\nv = C-v\nz = C-x')
