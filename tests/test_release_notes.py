from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
import tempfile
from unittest import mock


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

    def test_release_tags_reject_git_options_and_non_release_refs(self):
        module = load_release_notes()
        for tag in ("--all", "../HEAD", "0.4.1..HEAD", "0.4.1+bad/ref"):
            with self.subTest(tag=tag), self.assertRaises(SystemExit):
                module.tag_ref(tag)
        self.assertEqual(module.tag_ref("v0.5.0-dev.1"), "refs/tags/v0.5.0-dev.1")

    def test_build_replaces_output_symlink_without_overwriting_its_target(self):
        module = load_release_notes()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / "personal.md"
            target.write_text("personal content")
            output = root / "release.md"
            output.symlink_to(target)
            with mock.patch.object(module, "previous_tag", return_value=None), \
                    mock.patch.object(module, "commit_lines", return_value=[]):
                module.build(VERSION, output)
            self.assertFalse(output.is_symlink())
            self.assertIn("What's changed", output.read_text())
            self.assertEqual(target.read_text(), "personal content")
            self.assertFalse(list(root.glob(".release-notes-*")))

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
