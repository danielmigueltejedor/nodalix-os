import re
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_TEXT = (ROOT / ".github/workflows/publish-repo.yml").read_text(encoding="utf-8")
WORKFLOW = yaml.safe_load(WORKFLOW_TEXT)
SCRIPT = (ROOT / "tools/ci-publish-repo.sh").read_text(encoding="utf-8")
FINGERPRINT = "3DE029728CDE1828E18B437930D356AC0F6AB1CF"
# PyYAML reads the bare key "on" as the boolean True.
TRIGGERS = WORKFLOW.get("on", WORKFLOW.get(True))


class RepoPublishingTests(unittest.TestCase):
    def test_workflow_only_runs_when_a_person_starts_it(self):
        self.assertEqual(set(TRIGGERS), {"workflow_dispatch"})

    def test_deploy_job_alone_gets_pages_permissions(self):
        for name, job in WORKFLOW["jobs"].items():
            permissions = job.get("permissions", {})
            wants_pages = permissions.get("pages") == "write" or permissions.get("id-token") == "write"
            self.assertEqual(wants_pages, name == "deploy", name)

    def test_signing_runs_only_on_main_inside_a_protected_environment(self):
        build = WORKFLOW["jobs"]["build"]
        self.assertEqual(build["if"], "github.ref == 'refs/heads/main'")
        self.assertEqual(build["environment"], "nodalix-repo-signing")

    def test_candidate_run_is_validated_before_signing(self):
        steps = WORKFLOW["jobs"]["build"]["steps"]
        names = [step.get("name", step.get("uses", "")) for step in steps]
        check = next(i for i, n in enumerate(names) if n.startswith("Require a successful CI run"))
        download = next(i for i, n in enumerate(names) if n.startswith("actions/download-artifact"))
        self.assertLess(check, download)
        script = steps[check]["run"]
        for condition in ('.head_branch == "main"', '.conclusion == "success"', ".head_repository.full_name"):
            self.assertIn(condition, script)

    def test_private_key_comes_from_secrets_and_is_checked_against_the_keyring(self):
        self.assertIn("secrets.NODALIX_REPO_GPG_KEY", WORKFLOW_TEXT)
        self.assertIn(f"expected={FINGERPRINT}", SCRIPT)
        keyring = (ROOT / "packaging/nodalix-keyring/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn(FINGERPRINT, keyring)

    def test_pages_size_budget_is_enforced(self):
        self.assertRegex(SCRIPT, r"max_bytes=.*900")
        self.assertIn("Pages", SCRIPT)

    def test_third_party_actions_are_pinned_to_major_versions(self):
        for line in re.findall(r"uses: (\S+)", WORKFLOW_TEXT):
            self.assertRegex(line, r"@v\d+$")


if __name__ == "__main__":
    unittest.main()
