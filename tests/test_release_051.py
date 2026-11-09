"""Regression coverage for the 0.5.1 updater and packaging fixes."""

from __future__ import annotations

import hashlib
import importlib.machinery
import importlib.util
import platform
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
# Prefer the worktree migrations over any installed /usr/lib/nodalix-updater copy
# before nodalix-updater prepends that system path.
sys.path.insert(0, str(ROOT / "updater"))
from migrations.registry import (  # noqa: E402
    MigrationError,
    file_owned_by_package,
    remove_unowned_legacy_files,
)
from migrations.to_0_5_1 import (  # noqa: E402
    RELEASE_KEY_FILES,
    applies_release_key_cleanup,
    has_unowned_release_keys,
)

SPEC = importlib.util.spec_from_loader(
    "nodalix_updater_051",
    importlib.machinery.SourceFileLoader("nodalix_updater_051", str(ROOT / "updater/nodalix-updater")),
)
UPDATER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(UPDATER)


def component(component_id: str = "nodalix-release", required: bool = True, version: str = "0.5.0") -> dict:
    package = "nodalix-release" if component_id == "nodalix-release" else component_id.replace("_", "-")
    payload = component_id.encode()
    return {
        "id": component_id,
        "name": component_id,
        "package": package,
        "version": version,
        "asset": f"{package}-{version}-1-any.pkg.tar.zst",
        "sha256": hashlib.sha256(payload).hexdigest(),
        "size": len(payload),
        "required": required,
        "restart": "none",
    }


def manifest(components: list[dict] | None = None, version: str = "0.5.0") -> dict:
    return {
        "schema": 1,
        "schema_version": 1,
        "version": version,
        "minimum_version": "0.4.0",
        "channel": "stable",
        "arch": platform.machine() if platform.machine() in {"x86_64", "aarch64"} else "x86_64",
        "shell_restart_required": False,
        "reboot_required": False,
        "components": components or [component(), component("nodalix-store"), component("nodalix-updater")],
    }


def gh_release(tag: str, assets: list[str] | None = None) -> dict:
    names = assets or ["nodalix-manifest.json"]
    return {
        "tag_name": tag,
        "name": tag,
        "draft": False,
        "prerelease": False,
        "published_at": "2026-11-01T00:00:00Z",
        "html_url": f"https://github.com/example/nodalix-os/releases/tag/{tag}",
        "body": "",
        "assets": [{"name": name, "browser_download_url": f"https://example.test/{name}"} for name in names],
    }


class IncompleteInstallStatusTests(unittest.TestCase):
    def test_matching_version_with_missing_required_packages_is_repair_needed(self) -> None:
        value = manifest()
        release = gh_release("0.5.0")
        installed = {"nodalix-release"}

        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.5.0"),
            mock.patch.object(UPDATER, "list_releases", return_value=[release]),
            mock.patch.object(UPDATER, "manifest_for", return_value=value),
            mock.patch.object(UPDATER, "package_installed", side_effect=lambda name: name in installed),
        ):
            info = UPDATER.release_info(
                {"repository": "owner/repo", "channel": "stable", "manifest_asset": "nodalix-manifest.json"}
            )

        self.assertEqual(info["status"], "repair_needed")
        self.assertTrue(info["update_available"])
        self.assertIn("nodalix-store", info["missing_required"])
        self.assertIn("nodalix-updater", info["missing_required"])
        UPDATER.fail_closed(info, "update")

    def test_matching_version_with_all_required_packages_is_up_to_date(self) -> None:
        value = manifest()
        release = gh_release("0.5.0")

        with (
            mock.patch.object(UPDATER, "current_version", return_value="0.5.0"),
            mock.patch.object(UPDATER, "list_releases", return_value=[release]),
            mock.patch.object(UPDATER, "manifest_for", return_value=value),
            mock.patch.object(UPDATER, "package_installed", return_value=True),
        ):
            info = UPDATER.release_info(
                {"repository": "owner/repo", "channel": "stable", "manifest_asset": "nodalix-manifest.json"}
            )

        self.assertEqual(info["status"], "up_to_date")
        self.assertFalse(info["update_available"])
        self.assertEqual(info["missing_required"], [])

    def test_do_update_continues_for_repair_needed_without_force(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        originals = (
            UPDATER.RELEASE_PATH,
            UPDATER.STATE_DIR,
            UPDATER.CACHE_DIR,
            UPDATER.HISTORY_PATH,
            UPDATER.STATUS_PATH,
            UPDATER.LOCK_PATH,
            UPDATER.PACMAN_LOCK_PATH,
        )
        UPDATER.RELEASE_PATH = root / "nodalix-release"
        UPDATER.STATE_DIR = root / "state"
        UPDATER.CACHE_DIR = root / "cache"
        UPDATER.HISTORY_PATH = UPDATER.STATE_DIR / "history.jsonl"
        UPDATER.STATUS_PATH = UPDATER.STATE_DIR / "status.json"
        UPDATER.LOCK_PATH = UPDATER.STATE_DIR / "update.lock"
        UPDATER.PACMAN_LOCK_PATH = root / "pacman-db.lck"
        UPDATER.RELEASE_PATH.write_text('VERSION_ID="0.5.0"\n', encoding="utf-8")

        value = manifest([component()])
        info = {
            "status": "repair_needed",
            "update_available": True,
            "missing_required": ["nodalix-release"],
            "latest_version": "0.5.0",
            "candidate_version": "0.5.0",
            "channel": "stable",
            "_manifest": value,
            "_release": {
                "tag_name": "0.5.0",
                "assets": [
                    {"name": component()["asset"], "browser_download_url": "fixture://nodalix-release"},
                ],
            },
        }

        def download(url: str, destination: Path) -> None:
            destination.write_bytes(url.removeprefix("fixture://").encode())

        def run(command: list[str], **kwargs: object) -> mock.Mock:
            if command[:2] == ["pacman", "-Qo"]:
                return mock.Mock(returncode=1, stdout="", stderr="")
            if command[:1] == ["systemctl"]:
                return mock.Mock(returncode=0)
            if command[:1] == ["bsdtar"]:
                return mock.Mock(
                    returncode=0,
                    stdout="pkgname = nodalix-release\npkgver = 0.5.0-1\n",
                    stderr="",
                )
            self.assertEqual(command[0], "pacman")
            self.assertIn("-U", command)
            self.assertNotIn("--overwrite", command)
            return mock.Mock(returncode=0, stdout="")

        try:
            with (
                mock.patch.object(UPDATER.os, "geteuid", return_value=0),
                mock.patch.object(UPDATER, "release_info", return_value=info),
                mock.patch.object(UPDATER, "run_migrations", return_value=[]),
                mock.patch.object(UPDATER, "run_package_preflight", return_value=[]),
                mock.patch.object(UPDATER, "prepare_gnome_dependencies", return_value=None),
                mock.patch.object(UPDATER, "backup_gnome_transition", return_value=None),
                mock.patch.object(UPDATER, "package_installed", return_value=False),
                mock.patch.object(UPDATER, "download_to", side_effect=download),
                mock.patch.object(UPDATER.subprocess, "run", side_effect=run) as runner,
            ):
                result = UPDATER.do_update({"repository": "owner/repo", "channel": "stable"}, force=False)
            pacman_u = [
                call
                for call in runner.call_args_list
                if call.args and call.args[0] and call.args[0][0] == "pacman" and "-U" in call.args[0]
            ]
            self.assertEqual(len(pacman_u), 1)
            self.assertEqual(result["state"], "completed")
        finally:
            (
                UPDATER.RELEASE_PATH,
                UPDATER.STATE_DIR,
                UPDATER.CACHE_DIR,
                UPDATER.HISTORY_PATH,
                UPDATER.STATUS_PATH,
                UPDATER.LOCK_PATH,
                UPDATER.PACMAN_LOCK_PATH,
            ) = originals
            temporary.cleanup()


class ReleaseKeyMigrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_unowned_release_key_is_cleared(self) -> None:
        key = self.root / "usr" / "share" / "nodalix" / "keys" / "release.pub"
        key.parent.mkdir(parents=True)
        key.write_text("dev-key\n", encoding="utf-8")
        files = (str(key),)

        self.assertTrue(has_unowned_release_keys(files=files, owned_check=lambda path: False))
        self.assertTrue(
            applies_release_key_cleanup(
                "0.5.0",
                "0.5.1",
                UPDATER.compare_versions,
                owned_check=lambda path: False,
                files=files,
            )
        )

        payload = remove_unowned_legacy_files(
            files=files,
            backup_dir=self.root / "state" / "migrations" / "0.5.0-release-key",
            owned_check=lambda path: False,
            migration_id="0.5.0-release-key",
        )
        self.assertEqual(payload["id"], "0.5.0-release-key")
        self.assertFalse(key.exists())
        self.assertEqual(len(payload["removed"]), 1)
        backed = Path(payload["removed"][0]["backup"])
        self.assertEqual(backed.read_text(encoding="utf-8"), "dev-key\n")

    def test_owned_release_key_is_left_alone(self) -> None:
        key = self.root / "release.pub"
        key.write_text("owned\n", encoding="utf-8")
        self.assertFalse(has_unowned_release_keys(files=(str(key),), owned_check=lambda path: True))
        self.assertFalse(
            applies_release_key_cleanup(
                "0.5.0",
                "0.5.1",
                UPDATER.compare_versions,
                owned_check=lambda path: True,
                files=(str(key),),
            )
        )
        self.assertTrue(key.exists())

    def test_release_key_paths_are_explicit(self) -> None:
        self.assertEqual(
            set(RELEASE_KEY_FILES),
            {
                "/usr/share/nodalix/keys/release.pub",
                "/usr/share/nodalix/keys/release.pub.pre-0.5.0",
            },
        )

    def test_pacman_no_package_owns_is_treated_as_unowned(self) -> None:
        path = Path("/usr/share/nodalix/keys/release.pub")

        class Unowned:
            returncode = 1
            stdout = ""
            stderr = f"error: No package owns {path}\n"

        self.assertFalse(
            file_owned_by_package(path, pacman=lambda *args, **kwargs: Unowned())
        )

        class UnownedSpanish:
            returncode = 1
            stdout = ""
            stderr = f"error: Ningún paquete contiene {path}\n"

        self.assertFalse(
            file_owned_by_package(path, pacman=lambda *args, **kwargs: UnownedSpanish())
        )

        class Locked:
            returncode = 1
            stdout = ""
            stderr = "error: failed to synchronize all databases (unable to lock database)\n"

        with self.assertRaises(MigrationError):
            file_owned_by_package(path, pacman=lambda *args, **kwargs: Locked())


class StoreConflictPackagingTests(unittest.TestCase):
    def test_store_declares_bazaar_conflict_and_replace(self) -> None:
        pkgbuild = (ROOT / "packaging/nodalix-store/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn("conflicts=('bazaar')", pkgbuild)
        self.assertIn("replaces=('bazaar')", pkgbuild)

    def test_updater_always_ships_release_public_key(self) -> None:
        pkgbuild = (ROOT / "packaging/nodalix-updater/PKGBUILD").read_text(encoding="utf-8")
        self.assertIn(
            'install -Dm644 "$srcdir/updater/release.pub" "$pkgdir/usr/share/nodalix/keys/release.pub"',
            pkgbuild,
        )

    def test_release_workflow_refuses_published_stable_uploads(self) -> None:
        workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        self.assertIn("tools/release_upload_policy.py", workflow)
        self.assertIn("nodalix-release-publish-", workflow)
        self.assertIn("upload_flags+=(--clobber)", workflow)

        loader = importlib.machinery.SourceFileLoader(
            "release_upload_policy",
            str(ROOT / "tools/release_upload_policy.py"),
        )
        policy = importlib.util.module_from_spec(
            importlib.util.spec_from_loader(loader.name, loader)
        )
        loader.exec_module(policy)

        self.assertFalse(policy.allow_asset_upload(is_draft=False, is_prerelease=False))
        self.assertTrue(policy.allow_asset_upload(is_draft=True, is_prerelease=False))
        self.assertTrue(policy.allow_asset_upload(is_draft=False, is_prerelease=True))
        self.assertTrue(policy.allow_asset_upload(is_draft=True, is_prerelease=True))

        published = '{"isDraft": false, "isPrerelease": false, "assets": [], "tagName": "0.5.1"}'
        with mock.patch.object(policy.sys, "stderr", new=mock.Mock()):
            self.assertEqual(
                policy.main(["--release-json", published, "--tag", "0.5.1"]),
                1,
            )
        draft = '{"isDraft": true, "isPrerelease": false, "assets": [], "tagName": "0.5.1"}'
        self.assertEqual(policy.main(["--release-json", draft, "--tag", "0.5.1"]), 0)
        prerelease = '{"isDraft": false, "isPrerelease": true, "assets": [], "tagName": "0.5.1-rc.1"}'
        self.assertEqual(
            policy.main(["--release-json", prerelease, "--tag", "0.5.1-rc.1"]),
            0,
        )
        self.assertIn("return it to draft first", (ROOT / "tools/release_upload_policy.py").read_text(encoding="utf-8"))


class NowPlayingMultiMonitorToolingTests(unittest.TestCase):
    def test_patcher_is_idempotent_on_synthetic_extension(self) -> None:
        fixture = """
class MprisSource extends Signals.EventEmitter {
}

class MediaModel {
    constructor(settings, {keepStackVisible, divided, closeMenu}) {
        this._settings = settings;
        this._players = new Set();
        this._shown = new Set();

        this._analyzer = new SpectrumAnalyzer();
        this.equalizer = new EqualizerIcon();
        this.stack = new CardStack(closeMenu, keepStackVisible, divided);

        this._source = new MprisSource();
    }

    destroy() {
        this._stopListening();
        this._source?.destroy();
        this._source = null;
    }
}

class NowPlayingButton {
    constructor(settings) {
        this._model = new MediaModel(settings, {
            keepStackVisible: true,
            divided: true,
            closeMenu: () => this.menu.close(),
        });
    }
}

// The shell shows its own media controls in the notification list
class BuiltinMediaHider {
    hide() {}
    restore() {}
    destroy() { this.restore(); }
}

export default class NowPlayingExtension extends Extension {
    enable() {
        this._hider = new BuiltinMediaHider();
        this._build();
        this._syncBuiltinMedia();
    }

    disable() {
        this._settings?.disconnectObject(this);
        this._settings = null;
        this._destroyHost();
        this._hider?.destroy();
        this._hider = null;
    }

    _syncBuiltinMedia() {
        if (readSetting(this._settings, 'hide-builtin-media'))
            this._hider?.hide();
        else
            this._hider?.restore();
    }

    _rebuild() {
        this._destroyHost();
        this._build();
    }
}
"""
        with tempfile.TemporaryDirectory() as raw:
            module_dir = Path(raw) / "nowplaying@epogonii.github.io"
            module_dir.mkdir()
            path = module_dir / "extension.js"
            path.write_text(fixture, encoding="utf-8")
            loader = importlib.machinery.SourceFileLoader(
                "patch_nowplaying_multimonitor",
                str(ROOT / "tools/patch-nowplaying-multimonitor.py"),
            )
            module = importlib.util.module_from_spec(importlib.util.spec_from_loader(loader.name, loader))
            loader.exec_module(module)
            module.patch(path)
            first = path.read_text(encoding="utf-8")
            module.patch(path)
            second = path.read_text(encoding="utf-8")
            self.assertEqual(first, second)
            self.assertIn("acquireSharedMprisSource", first)
            self.assertIn("_attachSecondaryHosts", first)
            self.assertIn("_allMessageViews", first)
            self.assertIn("secondaryMonitorPanels", first)
            self.assertIn("_nodalixMultiMonitorPanels", first)
            self.assertIn("_nodalixMmPanelBus", first)
            self.assertNotIn("Main.mmPanel ??", first)
            self.assertNotIn("(Main.mmPanel", first)
            with self.assertRaises(SystemExit):
                module.patch(Path(raw) / "extension.js")
            outside = Path(raw) / "other" / "extension.js"
            outside.parent.mkdir()
            outside.write_text(fixture, encoding="utf-8")
            with self.assertRaises(SystemExit):
                module.patch(outside)

    def test_multi_monitor_center_row_and_nowplaying_exclusion(self) -> None:
        panel = (ROOT / "gnome/extensions/multi-monitors-bar@frederykabryan/mmpanel.js").read_text(encoding="utf-8")
        policy = (
            ROOT / "gnome/extensions/multi-monitors-bar@frederykabryan/monitorIndicatorPolicy.js"
        ).read_text(encoding="utf-8")
        self.assertIn("this._centerRow", panel)
        self.assertIn("nowplaying@epogonii.github.io", policy)
        build = (ROOT / "tools/build-nodalix-shell.sh").read_text(encoding="utf-8")
        self.assertIn("patch-nowplaying-multimonitor.py", build)


if __name__ == "__main__":
    unittest.main()
