from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class SecurityDashboardTests(unittest.TestCase):
    def test_security_probe_is_packaged_and_read_only(self) -> None:
        helper = ROOT / "shell/scripts/nodalix-security-status.py"
        pkgbuild = (
            ROOT / "packaging/nodalix-shell/PKGBUILD"
        ).read_text(encoding="utf-8")
        source = helper.read_text(encoding="utf-8")

        self.assertTrue(helper.is_file())
        self.assertIn(
            'nodalix-security-status.py" "$pkgdir/usr/bin/nodalix-security-status"',
            pkgbuild,
        )
        self.assertNotIn("sudo", source)
        self.assertNotIn("shell=True", source)

    def test_security_probe_covers_dashboard_sources(self) -> None:
        source = (
            ROOT / "shell/scripts/nodalix-security-status.py"
        ).read_text(encoding="utf-8")

        self.assertIn("ufw.service", source)
        self.assertIn("/sys/firmware/efi", source)
        self.assertIn('["pacman", "-Qm"]', source)
        self.assertIn("archlinux-keyring", source)
        self.assertIn('"--show-permissions"', source)
        self.assertIn("nodalix-*.service", source)
        self.assertIn("NoNewPrivileges", source)
        self.assertIn("ProtectSystem", source)

    def test_security_service_is_registered(self) -> None:
        service = (
            ROOT / "shell/services/SecurityService.qml"
        ).read_text(encoding="utf-8")
        qmldir = (
            ROOT / "shell/services/qmldir"
        ).read_text(encoding="utf-8")

        self.assertIn(
            "singleton SecurityService        1.0 SecurityService.qml",
            qmldir,
        )
        self.assertIn(
            'command: ["/usr/bin/nodalix-security-status"]',
            service,
        )
        self.assertIn("property bool secureBootEnabled", service)
        self.assertIn("property int flatpakBroadCount", service)
        self.assertIn("property var flatpakBroadApps", service)
        self.assertIn("flatpakReasonText", service)
        self.assertIn("property int servicesWithoutHardeningCount", service)


    def test_security_navigation_lives_under_system(self) -> None:
        settings = (
            ROOT / "shell/panels/Settings.qml"
        ).read_text(encoding="utf-8")

        personal = settings.split(
            '"personal-group": [',
            1,
        )[1].split(
            '"appearance-group": [',
            1,
        )[0]
        system = settings.split(
            '"system-group": [',
            1,
        )[1].split(
            '    })',
            1,
        )[0]

        self.assertNotIn('id: "security"', personal)
        self.assertIn('id: "security"', system)
        self.assertIn(
            'I18n.tr("Firewall, Secure Boot and system protection")',
            system,
        )

    def test_updates_keep_compact_aur_copy_and_aligned_action_columns(self) -> None:
        settings = (
            ROOT / "shell/panels/Settings.qml"
        ).read_text(encoding="utf-8")

        updates = settings.split(
            '// Updates',
            1,
        )[1].split(
            '// Desktop widgets',
            1,
        )[0]

        self.assertIn(
            'I18n.tr("AUR packages · manual review")',
            updates,
        )
        self.assertIn("Layout.preferredWidth: 112", updates)
        self.assertIn("Layout.preferredWidth: 152", updates)
        self.assertIn("id: _updateAction", updates)
        self.assertIn("Text.ElideRight", updates)

    def test_flatpak_permission_review_is_explanatory(self) -> None:
        helper = (
            ROOT / "shell/scripts/nodalix-security-status.py"
        ).read_text(encoding="utf-8")
        settings = (
            ROOT / "shell/panels/Settings.qml"
        ).read_text(encoding="utf-8")

        for reason in (
            "host-filesystem",
            "home-filesystem",
            "all-devices",
            "session-bus",
            "system-bus",
        ):
            self.assertIn(reason, helper)

        self.assertIn(
            'I18n.tr("Flatpak permission review")',
            settings,
        )
        self.assertIn(
            "SecurityService.flatpakBroadApps",
            settings,
        )
        self.assertIn(
            "SecurityService.flatpakReasonText(modelData.reasons)",
            settings,
        )
        self.assertIn(
            "does not automatically mean they are unsafe",
            settings,
        )

    def test_security_settings_keeps_locking_and_adds_dashboard(self) -> None:
        settings = (
            ROOT / "shell/panels/Settings.qml"
        ).read_text(encoding="utf-8")

        security = settings.split(
            "// Security",
            1,
        )[1].split(
            "// Appearance",
            1,
        )[0]

        self.assertIn('I18n.tr("System security")', security)
        self.assertIn("SecurityService.refresh()", security)
        self.assertIn('I18n.tr("Firewall")', security)
        self.assertIn('I18n.tr("Secure Boot")', security)
        self.assertIn('I18n.tr("Package signatures")', security)
        self.assertIn('I18n.tr("External / AUR packages")', security)
        self.assertIn('I18n.tr("Flatpak permissions")', security)
        self.assertIn('I18n.tr("Service hardening")', security)

        # Existing session protection remains part of the same Security page.
        self.assertIn('I18n.tr("Automatic screen lock")', security)
        self.assertIn('"security.autoLock.enabled"', security)


if __name__ == "__main__":
    unittest.main()
