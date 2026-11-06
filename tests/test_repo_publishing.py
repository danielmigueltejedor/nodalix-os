import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = (ROOT / ".github/workflows/publish-repo.yml").read_text(encoding="utf-8")
SCRIPT = (ROOT / "tools/ci-publish-repo.sh").read_text(encoding="utf-8")
FINGERPRINT = "3DE029728CDE1828E18B437930D356AC0F6AB1CF"


class RepoPublishingTests(unittest.TestCase):
    def test_workflow_only_runs_when_a_person_starts_it(self):
        triggers = WORKFLOW.split("jobs:")[0]
        self.assertIn("workflow_dispatch:", triggers)
        for automatic in ("push:", "pull_request", "schedule:"):
            self.assertNotIn(automatic, triggers)

    def test_deploy_job_alone_gets_pages_permissions(self):
        build, deploy = WORKFLOW.split("  deploy:")
        self.assertNotIn("pages: write", build)
        self.assertNotIn("id-token: write", build)
        self.assertIn("pages: write", deploy)
        self.assertIn("id-token: write", deploy)

    def test_private_key_comes_from_secrets_and_is_checked_against_the_keyring(self):
        self.assertIn("secrets.NODALIX_REPO_GPG_KEY", WORKFLOW)
        self.assertIn(f"expected={FINGERPRINT}", SCRIPT)
        keyring = (ROOT / "packaging/nodalix-keyring/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn(FINGERPRINT, keyring)

    def test_pages_size_budget_is_enforced(self):
        self.assertRegex(SCRIPT, r"max_bytes=.*900")
        self.assertIn("Pages", SCRIPT)

    def test_third_party_actions_are_pinned_to_major_versions(self):
        for line in re.findall(r"uses: (\S+)", WORKFLOW):
            self.assertRegex(line, r"@v\d+$")


if __name__ == "__main__":
    unittest.main()
