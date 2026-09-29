from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReleaseUpgradePolicyTests(unittest.TestCase):
    def test_real_upgrade_script_never_uses_overwrite(self) -> None:
        script = (
            ROOT
            / "tools"
            / "test-package-upgrade.sh"
        ).read_text(encoding="utf-8")

        self.assertIn("pacman_root", script)
        self.assertIn("-Udd", script)
        self.assertIn("--noscriptlet", script)
        self.assertNotIn("--overwrite", script)

    def test_release_requires_real_upgrade_job(self) -> None:
        workflow = (
            ROOT
            / ".github"
            / "workflows"
            / "release.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("  upgrade-test:", workflow)
        self.assertIn(
            "needs.upgrade-test.result == 'success'",
            workflow,
        )

    def test_iso_waits_for_upgrade_validation(self) -> None:
        workflow = (
            ROOT
            / ".github"
            / "workflows"
            / "release.yml"
        ).read_text(encoding="utf-8")

        iso = workflow.split("  iso:", 1)[1].split(
            "  publish:",
            1,
        )[0]

        self.assertIn("- upgrade-test", iso)

    def test_upgrade_ci_tests_multiple_stable_origins(self) -> None:
        workflow = (
            ROOT
            / ".github"
            / "workflows"
            / "release.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("tail -n 2", workflow)
        self.assertIn("legacy-cursor", workflow)
        self.assertIn("nodalix-cursor-theme-*.pkg.tar.zst", workflow)
        self.assertIn("test-package-upgrade.sh", workflow)

    def test_upgrade_script_executes_cursor_migration(self) -> None:
        script = (
            ROOT
            / "tools"
            / "test-package-upgrade.sh"
        ).read_text(encoding="utf-8")

        self.assertIn("migrate-cursor-theme", script)
        self.assertIn('NODALIX_ROOT="$ROOT"', script)
        self.assertIn("legacy-Nodalix-*", script)
        self.assertIn(
            "/usr/share/nodalix/cursor-theme/Nodalix",
            script,
        )


if __name__ == "__main__":
    unittest.main()
