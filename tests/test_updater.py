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


def component(component_id: str = "nodalix-release", required: bool = True) -> dict:
    package = component_id
    payload = package.encode()
    return {
        "id": component_id,
        "name": component_id,
        "package": package,
        "version": "0.2.0",
        "asset": f"{package}-0.2.0-1-any.pkg.tar.zst",
        "sha256": hashlib.sha256(payload).hexdigest(),
        "size": len(payload),
        "required": required,
        "restart": "shell" if component_id == "nodalix-shell" else "none",
    }


def manifest(components: list[dict] | None = None) -> dict:
    return {
        "schema": 1,
        "schema_version": 1,
        "version": "0.2.0",
        "minimum_version": "0.1.1",
        "channel": "stable",
        "shell_restart_required": False,
        "reboot_required": False,
        "components": components or [component()],
    }


class VersionTests(unittest.TestCase):
    def test_semantic_version_order(self) -> None:
        self.assertGreater(UPDATER.compare_versions("0.2.0", "0.2.0-rc.2"), 0)
        self.assertGreater(UPDATER.compare_versions("0.2.0-rc.10", "0.2.0-rc.2"), 0)
        self.assertLess(UPDATER.compare_versions("0.1.1", "0.2.0"), 0)
        self.assertEqual(UPDATER.compare_versions("v0.2.0", "0.2.0"), 0)

    def test_rejects_non_semver(self) -> None:
        with self.assertRaises(UPDATER.ManifestError):
            UPDATER.parse_version("0.2")


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

    def test_optional_component_follows_installed_state(self) -> None:
        optional = component("nodalix-vinilo", required=False)
        value = manifest([component(), optional])
        with mock.patch.object(UPDATER, "package_installed", side_effect=lambda name: name == "nodalix-vinilo"):
            self.assertEqual([item["id"] for item in UPDATER.selected_components(value)], ["nodalix-release", "nodalix-vinilo"])
        with mock.patch.object(UPDATER, "package_installed", return_value=False):
            self.assertEqual([item["id"] for item in UPDATER.selected_components(value)], ["nodalix-release"])


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
            "_manifest": value,
            "_release": {"assets": assets},
        }

    def download(self, url: str, destination: Path) -> None:
        destination.write_bytes(url.removeprefix("fixture://").encode())

    def test_installs_all_packages_in_one_pacman_transaction(self) -> None:
        value = manifest([component(), component("nodalix-updater"), component("nodalix-shell")])

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
        self.assertEqual(result["state"], "completed")
        self.assertTrue(result["shell_restart_required"])
        self.assertEqual(json.loads(UPDATER.STATUS_PATH.read_text())["state"], "completed")
        self.assertEqual(len(UPDATER.HISTORY_PATH.read_text().splitlines()), 1)

    def test_checksum_failure_prevents_pacman(self) -> None:
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


if __name__ == "__main__":
    unittest.main()
