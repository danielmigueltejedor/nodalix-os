from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
VERSION = (ROOT / "VERSION").read_text().strip()


def load_release_notes():
    path = ROOT / "tools" / "release_notes.py"
    spec = importlib.util.spec_from_file_location("nodalix_release_notes", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ReleaseNotesPolicyTests(unittest.TestCase):
    def test_current_version_has_curated_release_notes(self):
        expected = ROOT / "docs" / "releases" / f"v{VERSION}.md"
        self.assertTrue(
            expected.is_file(),
            f"Missing curated release notes: {expected.relative_to(ROOT)}",
        )

    def test_current_release_notes_pass_policy(self):
        module = load_release_notes()

        cwd = Path.cwd()
        try:
            import os

            os.chdir(ROOT)
            path, text = module.validate(VERSION)
        finally:
            os.chdir(cwd)

        self.assertEqual(
            path,
            Path("docs/releases") / f"v{VERSION}.md",
        )
        self.assertTrue(text.strip())

    def test_ci_requires_curated_release_notes(self):
        workflow = (
            ROOT / ".github" / "workflows" / "ci.yml"
        ).read_text()

        self.assertIn(
            "tools/release_notes.py validate",
            workflow,
        )
        self.assertIn(
            "--version",
            workflow,
        )

    def test_release_workflow_validates_and_builds_notes(self):
        workflow = (
            ROOT / ".github" / "workflows" / "release.yml"
        ).read_text()

        self.assertIn(
            "tools/release_notes.py validate",
            workflow,
        )
        self.assertIn(
            "tools/release_notes.py build",
            workflow,
        )
        self.assertIn(
            "dist/release-notes.md",
            workflow,
        )
        self.assertIn(
            '--notes-file "$NOTES_FILE"',
            workflow,
        )

    def test_release_workflow_has_no_generic_notes_fallback(self):
        workflow = (
            ROOT / ".github" / "workflows" / "release.yml"
        ).read_text()

        self.assertNotIn(
            "Paquetes oficiales de Nodalix OS",
            workflow,
        )


if __name__ == "__main__":
    unittest.main()
