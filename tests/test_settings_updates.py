from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class SettingsUpdatesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.settings = (
            ROOT / 'shell/panels/Settings.qml'
        ).read_text(encoding='utf-8')

    def test_updates_has_single_navigation_entry_under_system(self) -> None:
        applications = self.settings.split(
            '"applications-group": [',
            1,
        )[1].split(
            '"services-group": [',
            1,
        )[0]

        system = self.settings.split(
            '"system-group": [',
            1,
        )[1].split(
            '    })',
            1,
        )[0]

        self.assertNotIn(
            'id: "nodalix-updates"',
            applications,
        )

        self.assertNotIn(
            'id: "updates"',
            applications,
        )

        self.assertIn(
            'id: "updates"',
            system,
        )

    def test_unified_updates_page_contains_nodalix_and_system(self) -> None:
        pane = self.settings.split(
            '// Updates',
            1,
        )[1].split(
            '// Desktop widgets',
            1,
        )[0]

        self.assertIn('UpdateService.nodalixUpdateAvailable', pane)
        self.assertIn('UpdateService.systemUpdates', pane)
        self.assertIn('UpdateService.aurUpdates', pane)
        self.assertIn('UpdateService.flatpakUpdates', pane)
        self.assertIn('UpdateService.autoNodalix', pane)
        self.assertIn('UpdateService.autoSystem', pane)



    def test_release_notes_are_markdown_and_collapsible(self) -> None:
        settings = (
            ROOT / "shell/panels/Settings.qml"
        ).read_text(encoding="utf-8")

        self.assertIn("textFormat: Text.MarkdownText", settings)
        self.assertIn("Layout.maximumHeight: _releaseNotes.expanded", settings)
        self.assertIn("clip: !_releaseNotes.expanded", settings)
        self.assertIn("_releaseNotesText.implicitHeight > 120", settings)
        self.assertIn('"Show more"', settings)
        self.assertIn('"Show less"', settings)
        self.assertIn("Qt.openUrlExternally(link)", settings)


    def test_settings_qmldir_exports_recovery_settings(self) -> None:
        qmldir = (
            ROOT / "shell/panels/qmldir"
        ).read_text(encoding="utf-8")

        self.assertIn(
            "RecoverySettings  1.0 RecoverySettings.qml",
            qmldir,
        )


class HymissionDispatchTests(unittest.TestCase):
    def test_overview_uses_native_hymission_lua_api(self) -> None:
        script = (
            ROOT / "shell/scripts/nodalix-overview.py"
        ).read_text(encoding="utf-8")

        self.assertIn("hl.plugin.hymission.toggle()", script)
        self.assertIn("eval", script)
        self.assertNotIn("hymission:toggle", script)


if __name__ == '__main__':
    unittest.main()
