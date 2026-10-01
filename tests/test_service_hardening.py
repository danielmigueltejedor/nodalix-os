from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ServiceHardeningTests(unittest.TestCase):
    def _unit(self, relative: str) -> str:
        return (ROOT / relative).read_text(encoding="utf-8")

    def test_read_only_update_checker_is_strictly_sandboxed(self) -> None:
        unit = self._unit("updater/nodalix-update-check.service")

        for directive in (
            "NoNewPrivileges=yes",
            "PrivateTmp=yes",
            "PrivateDevices=yes",
            "ProtectSystem=strict",
            "ProtectHome=yes",
            "ProtectKernelTunables=yes",
            "ProtectKernelModules=yes",
            "ProtectControlGroups=yes",
            "ProtectClock=yes",
            "RestrictSUIDSGID=yes",
            "LockPersonality=yes",
            "CapabilityBoundingSet=",
        ):
            self.assertIn(directive, unit)

        self.assertIn(
            "ReadWritePaths=/var/cache/nodalix-updater /var/lib/nodalix-updater",
            unit,
        )

    def test_bluetooth_preparation_keeps_only_required_capabilities(self) -> None:
        unit = self._unit(
            "phone-link/systemd/nodalix-phone-link-bluetooth.service"
        )

        for directive in (
            "NoNewPrivileges=yes",
            "PrivateTmp=yes",
            "ProtectSystem=strict",
            "ProtectHome=yes",
            "ProtectKernelTunables=yes",
            "ProtectKernelModules=yes",
            "ProtectControlGroups=yes",
            "ProtectClock=yes",
            "RestrictSUIDSGID=yes",
            "LockPersonality=yes",
            "CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW",
        ):
            self.assertIn(directive, unit)

    def test_mutating_services_use_conservative_hardening(self) -> None:
        units = (
            "updater/nodalix-update-system.service",
            "updater/nodalix-update-nodalix.service",
            "updater/nodalix-recovery.service",
        )

        for relative in units:
            with self.subTest(unit=relative):
                unit = self._unit(relative)
                for directive in (
                    "NoNewPrivileges=yes",
                    "PrivateTmp=yes",
                    "ProtectHome=read-only",
                    "ProtectKernelTunables=yes",
                    "ProtectControlGroups=yes",
                    "ProtectClock=yes",
                    "LockPersonality=yes",
                ):
                    self.assertIn(directive, unit)

                # These jobs intentionally install/restore arbitrary system
                # package payloads. Do not block required filesystem writes or
                # creation/restoration of packaged SUID/SGID files.
                self.assertNotIn("ProtectSystem=strict", unit)
                self.assertNotIn("RestrictSUIDSGID=yes", unit)

    def test_firmware_client_is_hardened_without_hiding_devices(self) -> None:
        unit = self._unit("updater/nodalix-update-firmware.service")

        for directive in (
            "NoNewPrivileges=yes",
            "PrivateTmp=yes",
            "ProtectHome=yes",
            "ProtectKernelTunables=yes",
            "ProtectControlGroups=yes",
            "ProtectClock=yes",
            "LockPersonality=yes",
        ):
            self.assertIn(directive, unit)

        self.assertNotIn("PrivateDevices=yes", unit)
        self.assertNotIn("ProtectSystem=strict", unit)

    def test_legacy_bluetooth_cleanup_never_deletes_owned_units(self) -> None:
        install = (
            ROOT
            / "packaging/nodalix-phone-link/nodalix-phone-link.install"
        ).read_text(encoding="utf-8")

        self.assertIn(
            'unit="nodalix-iphone-bluetooth.service"',
            install,
        )
        self.assertIn(
            'systemctl disable --now "$unit"',
            install,
        )
        self.assertIn(
            'pacman -Qo "$path"',
            install,
        )
        self.assertIn(
            'rm -f -- "$path"',
            install,
        )
        self.assertLess(
            install.index('pacman -Qo "$path"'),
            install.index('rm -f -- "$path"'),
        )


if __name__ == "__main__":
    unittest.main()
