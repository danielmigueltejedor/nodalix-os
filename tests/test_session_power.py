"""Guard the trust boundary: power controls never dismiss authentication."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class SessionPower(unittest.TestCase):
    def test_power_stays_inside_lock_surface(self):
        source=(ROOT/'shell/LockScreen.qml').read_text()
        self.assertIn('WlSessionLockSurface {',source)
        self.assertIn('SessionPowerControls {',source)
        controls=(ROOT/'shell/components/SessionPowerControls.qml').read_text()
        self.assertNotIn('locked = false',controls)
        self.assertNotIn('LockService.submit',controls)
        self.assertIn('operation.command = ["systemctl", pending]',controls)
        self.assertIn('if (previewOnly)',controls)
        request=controls.split('function request(action) {',1)[1].split('function confirm()',1)[0]
        self.assertNotIn('running = true',request)
        self.assertIn('pending = action',request)
    def test_transfer_cache_is_writable_in_hardened_service(self):
        service=(ROOT/'phone-link/systemd/nodalix-phone-link.service').read_text()
        self.assertIn('ProtectHome=read-only',service)
        self.assertIn('CacheDirectory=nodalix/phone-link',service)
        self.assertIn('CacheDirectoryMode=0700',service)
