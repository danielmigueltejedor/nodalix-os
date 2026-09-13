from __future__ import annotations

import hashlib
import importlib.util
import importlib.machinery
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_loader(
    "nodalix_updater",
    importlib.machinery.SourceFileLoader("nodalix_updater", str(ROOT / "updater/nodalix-updater")),
)
UPDATER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(UPDATER)


def component(component_id: str = "nodalix-release", required: bool = True, version: str = "0.2.0") -> dict:
    package = "nodalix-release" if component_id == "nodalix-release" else component_id.replace("_", "-")
    if component_id == "updater":
        package = "nodalix-updater"
    elif component_id == "shell":
        package = "nodalix-shell"
    payload = component_id.encode()
    return {
        "id": component_id,
        "name": component_id,
        "package": package,
        "version": version,
        "asset": f"{package}-{version.replace('-', '')}-1-any.pkg.tar.zst",
        "sha256": hashlib.sha256(payload).hexdigest(),
        "size": len(payload),
        "required": required,
        "restart": "shell" if component_id in {"nodalix-shell", "shell"} else "none",
    }


def manifest(components: list[dict] | None = None, version: str = "0.2.0") -> dict:
    return {
        "schema": 1,
        "schema_version": 1,
        "version": version,
        "minimum_version": "0.1.1",
        "channel": "stable",
        "arch": "x86_64",
        "shell_restart_required": False,
        "reboot_required": False,
        "components": components or [component()],
    }


def gh_release(tag: str, *, draft: bool = False, prerelease: bool = False, assets: list[str] | None = None) -> dict:
    return {
        "tag_name": tag,
        "name": tag,
        "draft": draft,
        "prerelease": prerelease,
        "published_at": "2026-09-13T00:00:00Z",
        "html_url": f"https://github.com/example/nodalix-os/releases/tag/{tag}",
        "body": "",
        "assets": [{"name": name, "browser_download_url": f"https://example.test/{name}"} for name in assets or []],
    }


class VersionTests(unittest.TestCase):
    def test_semantic_version_order(self) -> None:
        self.assertLess(UPDATER.compare_versions("0.2.0-beta.1", "0.2.0-beta.2"), 0)
        self.assertLess(UPDATER.compare_versions("0.2.0-beta.2", "0.2.0-beta.10"), 0)
        self.assertLess(UPDATER.compare_versions("0.2.0-beta.10", "0.2.0-rc.1"), 0)
        self.assertLess(UPDATER.compare_versions("0.2.0-rc.1", "0.2.0"), 0)
        self.assertLess(UPDATER.compare_versions("0.2.0", "0.2.1-beta.1"), 0)
        self.assertGreater(UPDATER.compare_versions("0.2.0", "0.2.0-rc.2"), 0)
        self.assertGreater(UPDATER.compare_versions("0.2.0-rc.10", "0.2.0-rc.2"), 0)
        self.assertLess(UPDATER.compare_versions("0.1.1", "0.2.0"), 0)
        self.assertEqual(UPDATER.compare_versions("v0.2.0", "0.2.0"), 0)
        self.assertEqual(UPDATER.compare_versions("v0.2.0-beta.1", "0.2.0-beta.1"), 0)

    def test_rejects_non_semver(self) -> None:
        with self.assertRaises(UPDATER.ManifestError):
            UPDATER.parse_version("0.2")
        with self.assertRaises(UPDATER.ManifestError):
            UPDATER.parse_version("0.2.0-beta.01")


class ChannelSelectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.releases = [
            gh_release("0.1.1"),
            gh_release("0.2.0-beta.1", prerelease=True),
            gh_release("0.2.0-beta.2", prerelease=True),
            gh_release("0.2.0-beta.10", prerelease=True),
            gh_release("0.2.0-rc.1", prerelease=True),
            gh_release("0.2.0"),
            gh_release("0.2.1-beta.1", prerelease=True),
            gh_release("9.9.9", draft=True),
            gh_release("draft-beta", draft=True, prerelease=True),
        ]

    def selected(self, channel: str, releases: list[dict] | None = None) -> str | None:
        release = UPDATER.select_release(releases or self.releases, channel)
        return None if release is None else release["tag_name"]

    def test_stable_ignores_beta_and_rc(self) -> None:
        self.assertEqual(self.selected("stable"), "0.2.0")
        self.assertNotIn(
            self.selected("stable", [gh_release("0.2.0-beta.1", prerelease=True), gh_release("0.2.0-rc.1", prerelease=True)]),
            {"0.2.0-beta.1", "0.2.0-rc.1"},
        )

    def test_stable_selects_newest_stable(self) -> None:
        releases = [gh_release("0.2.0"), gh_release("0.2.1"), gh_release("0.3.0"), gh_release("0.2.1-beta.1", prerelease=True)]
        self.assertEqual(self.selected("stable", releases), "0.3.0")

    def test_stable_ignores_github_false_prerelease_when_semver_is_pre(self) -> None:
        releases = [gh_release("0.2.0-beta.1", prerelease=False), gh_release("0.1.1")]
        self.assertEqual(self.selected("stable", releases), "0.1.1")

    def test_beta_accepts_beta_rc_and_stable(self) -> None:
        self.assertEqual(self.selected("beta", [gh_release("0.2.0-beta.1", prerelease=True)]), "0.2.0-beta.1")
        self.assertEqual(self.selected("beta", [gh_release("0.2.0-rc.1", prerelease=True)]), "0.2.0-rc.1")
        self.assertEqual(self.selected("beta", [gh_release("0.2.0")]), "0.2.0")

    def test_beta_prefers_stable_over_rc_of_same_version(self) -> None:
        releases = [gh_release("0.2.0-rc.1", prerelease=True), gh_release("0.2.0")]
        self.assertEqual(self.selected("beta", releases), "0.2.0")

    def test_beta_prefers_beta_10_over_beta_2(self) -> None:
        releases = [gh_release("0.2.0-beta.2", prerelease=True), gh_release("0.2.0-beta.10", prerelease=True)]
        self.assertEqual(self.selected("beta", releases), "0.2.0-beta.10")

    def test_beta_prefers_newer_minor_prerelease(self) -> None:
        releases = [gh_release("0.2.0"), gh_release("0.2.1-beta.1", prerelease=True)]
        self.assertEqual(self.selected("beta", releases), "0.2.1-beta.1")

    def test_drafts_are_always_ignored(self) -> None:
        releases = [gh_release("1.0.0", draft=True), gh_release("0.2.0-beta.99", draft=True, prerelease=True), gh_release("0.1.1")]
        self.assertEqual(self.selected("stable", releases), "0.1.1")
        self.assertEqual(self.selected("beta", releases), "0.1.1")

    def test_beta_does_not_use_github_list_order(self) -> None:
        releases = [
            gh_release("0.2.0-beta.2", prerelease=True),
            gh_release("0.2.0-beta.10", prerelease=True),
            gh_release("0.2.0-beta.1", prerelease=True),
        ]
        self.assertEqual(self.selected("beta", releases), "0.2.0-beta.10")

    def test_v_prefix_is_normalized(self) -> None:
        releases = [gh_release("v0.2.0-beta.1", prerelease=True), gh_release("v0.2.0")]
        self.assertEqual(self.selected("stable", releases), "v0.2.0")
        self.assertEqual(self.selected("beta", releases), "v0.2.0")

    def test_rc_alias_maps_to_beta(self) -> None:
        self.assertEqual(UPDATER.normalize_channel("rc"), "beta")
        self.assertEqual(self.selected("rc"), "0.2.1-beta.1")

    def test_private_repo_downloads_use_api_asset_url(self) -> None:
        asset = {
            "url": "https://api.github.com/repos/owner/repo/releases/assets/1",
            "browser_download_url": "https://github.com/owner/repo/releases/download/0.2.0-beta.1/nodalix-manifest.json",
        }
        with mock.patch.dict(UPDATER.os.environ, {"GITHUB_TOKEN": "token"}):
            self.assertEqual(UPDATER.asset_download_url(asset), asset["url"])
        with mock.patch.dict(UPDATER.os.environ, {}, clear=False):
            UPDATER.os.environ.pop("GITHUB_TOKEN", None)
            self.assertEqual(UPDATER.asset_download_url(asset), asset["browser_download_url"])


class ManifestValidationTests(unittest.TestCase):
    def test_accepts_release_manifest(self) -> None:
        UPDATER.validate_manifest(manifest())

    def test_rejects_path_traversal_asset(self) -> None:
        value = manifest()
        value["components"][0]["asset"] = "../package.pkg.tar.zst"
        with self.assertRaisesRegex(UPDATER.ManifestError, "asset name"):
            UPDATER.validate_manifest(value)

    def test_rejects_bad_checksum(self) -> None:
        value = manifest()
        value["components"][0]["sha256"] = "bad"
        with self.assertRaisesRegex(UPDATER.ManifestError, "SHA-256"):
            UPDATER.validate_manifest(value)

    def test_rejects_wrong_schema(self) -> None:
        value = manifest()
        value["schema"] = 2
        with self.assertRaisesRegex(UPDATER.ManifestError, "Schema no soportado"):
            UPDATER.validate_manifest(value)

    def test_rejects_tag_mismatch_in_release_info(self) -> None:
        release = gh_release("0.2.0-beta.1", prerelease=True, assets=["nodalix-manifest.json"])
        value = manifest(version="0.2.0-beta.2")
        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.1.1"),
            mock.patch.object(UPDATER, "list_releases", return_value=[release]),
            mock.patch.object(UPDATER, "manifest_for", return_value=value),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
        ):
            info = UPDATER.release_info({"repository": "owner/repo", "channel": "beta", "manifest_asset": "nodalix-manifest.json"})
        self.assertEqual(info["status"], "invalid_manifest")
        self.assertIn("no coinciden", info["error"])

    def test_optional_component_follows_installed_state(self) -> None:
        optional = component("nodalix-vinilo", required=False)
        value = manifest([component(), optional])
        with mock.patch.object(UPDATER, "package_installed", side_effect=lambda name: name == "nodalix-vinilo"):
            self.assertEqual([item["id"] for item in UPDATER.selected_components(value)], ["nodalix-release", "nodalix-vinilo"])
        with mock.patch.object(UPDATER, "package_installed", return_value=False):
            self.assertEqual([item["id"] for item in UPDATER.selected_components(value)], ["nodalix-release"])

    def test_missing_manifest_status(self) -> None:
        release = gh_release("0.2.0-beta.1", prerelease=True, assets=[])
        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.1.1"),
            mock.patch.object(UPDATER, "list_releases", return_value=[release]),
        ):
            info = UPDATER.release_info({"repository": "owner/repo", "channel": "beta", "manifest_asset": "nodalix-manifest.json"})
        self.assertEqual(info["status"], "manifest_missing")
        self.assertEqual(info["candidate_version"], "0.2.0-beta.1")
        self.assertFalse(info["update_available"])
        self.assertIn("nodalix-manifest.json", info["error"])

    def test_invalid_json_manifest_status(self) -> None:
        release = gh_release("0.2.0-beta.1", prerelease=True, assets=["nodalix-manifest.json"])
        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.1.1"),
            mock.patch.object(UPDATER, "list_releases", return_value=[release]),
            mock.patch.object(UPDATER, "request", return_value=(b"{not json", {})),
        ):
            info = UPDATER.release_info({"repository": "owner/repo", "channel": "beta", "manifest_asset": "nodalix-manifest.json"})
        self.assertEqual(info["status"], "invalid_manifest")

    def test_stable_channel_has_no_compatible_beta_release(self) -> None:
        releases = [gh_release("0.2.0-beta.1", prerelease=True)]
        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.1.1"),
            mock.patch.object(UPDATER, "list_releases", return_value=releases),
        ):
            info = UPDATER.release_info({"repository": "owner/repo", "channel": "stable", "manifest_asset": "nodalix-manifest.json"})
        self.assertEqual(info["status"], "no_compatible_release")
        self.assertFalse(info["update_available"])


class UpdateTransactionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.original_paths = (
            UPDATER.RELEASE_PATH,
            UPDATER.STATE_DIR,
            UPDATER.CACHE_DIR,
            UPDATER.HISTORY_PATH,
            UPDATER.STATUS_PATH,
            UPDATER.LOCK_PATH,
        )
        UPDATER.RELEASE_PATH = self.root / "nodalix-release"
        UPDATER.STATE_DIR = self.root / "state"
        UPDATER.CACHE_DIR = self.root / "cache"
        UPDATER.HISTORY_PATH = UPDATER.STATE_DIR / "history.jsonl"
        UPDATER.STATUS_PATH = UPDATER.STATE_DIR / "status.json"
        UPDATER.LOCK_PATH = UPDATER.STATE_DIR / "update.lock"
        UPDATER.RELEASE_PATH.write_text('VERSION_ID="0.1.1"\n', encoding="utf-8")

    def tearDown(self) -> None:
        (
            UPDATER.RELEASE_PATH,
            UPDATER.STATE_DIR,
            UPDATER.CACHE_DIR,
            UPDATER.HISTORY_PATH,
            UPDATER.STATUS_PATH,
            UPDATER.LOCK_PATH,
        ) = self.original_paths
        self.temporary.cleanup()

    def info(self, value: dict) -> dict:
        assets = [
            {"name": item["asset"], "browser_download_url": f"fixture://{item['id']}"}
            for item in value["components"]
        ]
        return {
            "status": "update_available",
            "update_available": True,
            "channel": "beta",
            "_manifest": value,
            "_release": {"assets": assets, "tag_name": value["version"]},
        }

    def download(self, url: str, destination: Path) -> None:
        destination.write_bytes(url.removeprefix("fixture://").encode())

    def test_installs_all_packages_in_one_pacman_transaction(self) -> None:
        value = manifest([component(), component("updater"), component("shell")])

        def pacman(command: list[str], **kwargs: object) -> mock.Mock:
            self.assertEqual(command[:4], ["pacman", "-U", "--noconfirm", "--needed"])
            self.assertEqual(len(command[4:]), 3)
            UPDATER.RELEASE_PATH.write_text('VERSION_ID="0.2.0"\n', encoding="utf-8")
            return mock.Mock(returncode=0)

        with (
            mock.patch.object(UPDATER.os, "geteuid", return_value=0),
            mock.patch.object(UPDATER, "release_info", return_value=self.info(value)),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
            mock.patch.object(UPDATER, "download_to", side_effect=self.download),
            mock.patch.object(UPDATER.subprocess, "run", side_effect=pacman) as run,
        ):
            result = UPDATER.do_update({})

        self.assertEqual(run.call_count, 1)
        self.assertNotIn("nodalix-updater", " ".join(str(call) for call in run.call_args_list if call.args[0][:1] != ["pacman"]))
        self.assertEqual(result["state"], "completed")
        self.assertTrue(result["shell_restart_required"])
        self.assertEqual(json.loads(UPDATER.STATUS_PATH.read_text())["state"], "completed")
        self.assertEqual(len(UPDATER.HISTORY_PATH.read_text().splitlines()), 1)

    def test_self_update_replaces_updater_in_same_transaction(self) -> None:
        value = manifest([component(), component("updater")])
        seen: list[list[str]] = []

        def pacman(command: list[str], **kwargs: object) -> mock.Mock:
            seen.append(command)
            UPDATER.RELEASE_PATH.write_text('VERSION_ID="0.2.0"\n', encoding="utf-8")
            return mock.Mock(returncode=0)

        with (
            mock.patch.object(UPDATER.os, "geteuid", return_value=0),
            mock.patch.object(UPDATER, "release_info", return_value=self.info(value)),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
            mock.patch.object(UPDATER, "download_to", side_effect=self.download),
            mock.patch.object(UPDATER.subprocess, "run", side_effect=pacman),
        ):
            UPDATER.do_update({})

        self.assertEqual(len(seen), 1)
        joined = " ".join(seen[0])
        self.assertIn("nodalix-updater-", joined)
        self.assertIn("nodalix-release-", joined)

    def test_checksum_failure_prevents_pacman_and_version_change(self) -> None:
        value = manifest()

        def corrupt(url: str, destination: Path) -> None:
            destination.write_bytes(b"corrupt")

        with (
            mock.patch.object(UPDATER.os, "geteuid", return_value=0),
            mock.patch.object(UPDATER, "release_info", return_value=self.info(value)),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
            mock.patch.object(UPDATER, "download_to", side_effect=corrupt),
            mock.patch.object(UPDATER.subprocess, "run") as run,
        ):
            with self.assertRaises(UPDATER.IntegrityError):
                UPDATER.do_update({})
        run.assert_not_called()
        self.assertEqual(UPDATER.current_version(), "0.1.1")
        self.assertFalse(UPDATER.HISTORY_PATH.exists())

    def test_missing_component_asset_prevents_install(self) -> None:
        value = manifest()
        payload = self.info(value)
        payload["_release"]["assets"] = []
        with (
            mock.patch.object(UPDATER.os, "geteuid", return_value=0),
            mock.patch.object(UPDATER, "release_info", return_value=payload),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
            mock.patch.object(UPDATER.subprocess, "run") as run,
        ):
            with self.assertRaises(UPDATER.ManifestMissingError):
                UPDATER.do_update({})
        run.assert_not_called()
        self.assertEqual(UPDATER.current_version(), "0.1.1")

    def test_pacman_failure_does_not_record_history(self) -> None:
        value = manifest()

        def pacman(command: list[str], **kwargs: object) -> None:
            raise UPDATER.subprocess.CalledProcessError(1, command)

        with (
            mock.patch.object(UPDATER.os, "geteuid", return_value=0),
            mock.patch.object(UPDATER, "release_info", return_value=self.info(value)),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
            mock.patch.object(UPDATER, "download_to", side_effect=self.download),
            mock.patch.object(UPDATER.subprocess, "run", side_effect=pacman),
        ):
            with self.assertRaises(UPDATER.subprocess.CalledProcessError):
                UPDATER.do_update({})
        self.assertEqual(UPDATER.current_version(), "0.1.1")
        self.assertFalse(UPDATER.HISTORY_PATH.exists())


if __name__ == "__main__":
    unittest.main()
