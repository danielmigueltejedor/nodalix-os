import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]


class PhoneLePreparation(unittest.TestCase):
    def test_enables_address_resolution_before_ancs_is_visible_and_preserves_flags(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); trace=root/'trace'; mgmt=root/'btmgmt'; ctl=root/'bluetoothctl'
            mgmt.write_text('''#!/bin/sh
case "$1" in
 info) echo 'current settings: powered le';;
 class) [ "$2 $3" = "4 8" ];;
 get-flags) printf 'Supported Flags: 0x0000000f\\nCurrent Flags:   0x00000002\\n';;
 set-flags) printf '%s\\n' "$*" >> '''+shlex.quote(str(trace))+''';;
esac
''')
            ctl.write_text('''#!/bin/sh
case "$1" in
 devices) printf 'Device AA:BB:CC:DD:EE:11 fixture phone\\nDevice AA:BB:CC:DD:EE:22 other\\n';;
 info)
   if [ "$2" = AA:BB:CC:DD:EE:11 ]; then
     printf '  Modalias: bluetooth:v004Cp0010d0001\\n  AddressType: public\\n  UUID: 0000111f-0000-1000-8000-00805f9b34fb\\n  UUID: 00001132-0000-1000-8000-00805f9b34fb\\n'
   else
     printf '  AddressType: public\\n  UUID: 0000111f-0000-1000-8000-00805f9b34fb\\n'
   fi;;
esac
''')
            for p in (mgmt,ctl):p.chmod(0o700)
            script=(ROOT/'phone-link/systemd/prepare-bluetooth').read_text().replace('/usr/bin/btmgmt',str(mgmt)).replace('/usr/bin/bluetoothctl',str(ctl))
            result=subprocess.run(['/usr/bin/bash','-c',script],capture_output=True,text=True,timeout=8,check=True)
            self.assertEqual(trace.read_text().splitlines(),['set-flags -t 1 -f 0x00000006 AA:BB:CC:DD:EE:11'])
            self.assertIn('Bluetooth class prepared',result.stdout)
            self.assertIn('Address resolution enabled',result.stdout)
            self.assertNotIn('AA:BB:CC',result.stdout)
