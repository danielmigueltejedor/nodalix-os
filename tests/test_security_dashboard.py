from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class SecurityDashboardTests(unittest.TestCase):
    def test_security_probe_is_packaged_and_read_only(self) -> None:
        helper = ROOT / "integrations/scripts/nodalix-security-status.py"
        pkgbuild = (
            ROOT / "packaging/nodalix-integrations/PKGBUILD"
        ).read_text(encoding="utf-8")
        source = helper.read_text(encoding="utf-8")

        self.assertTrue(helper.is_file())
        self.assertIn(
            'security-status',
            pkgbuild,
        )
        self.assertNotIn("sudo", source)
        self.assertNotIn("shell=True", source)

    def test_security_probe_covers_dashboard_sources(self) -> None:
        source = (
            ROOT / "integrations/scripts/nodalix-security-status.py"
        ).read_text(encoding="utf-8")

        self.assertIn("ufw.service", source)
        self.assertIn("/sys/firmware/efi", source)
        self.assertIn('["pacman", "-Qm"]', source)
        self.assertIn("archlinux-keyring", source)
        self.assertIn('"--show-permissions"', source)
        self.assertIn("nodalix-*.service", source)
        self.assertIn("NoNewPrivileges", source)
        self.assertIn("ProtectSystem", source)








if __name__ == "__main__":
    unittest.main()
