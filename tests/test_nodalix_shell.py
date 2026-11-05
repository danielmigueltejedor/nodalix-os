"""Native Nodalix Shell overlay tests (does not modify the host)."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "prepare_nodalix_shell", ROOT / "tools/prepare-nodalix-shell.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def fixture(path: Path, version: str = "51.1") -> None:
    (path / "js/ui").mkdir(parents=True, exist_ok=True)
    (path / "js/extensions").mkdir(exist_ok=True)
    (path / "js/extensions/sharedInternals.js").write_text(
        MODULE.GETTEXT_DETECTION + "\n"
        "        const path = extensionLine.slice(extensionLine.indexOf(basePath));\n")
    (path / "js/meson.build").write_text("subdir('misc')\n")
    (path / "meson.build").write_text(
        f"project('gnome-shell', 'c',\n  version: '{version}',\n)\n"
        "mutter_api_version = '51'\n")
    (path / "js/ui/main.js").write_text(
        "import * as Panel from './panel.js';\n"
        "export async function start() {\n"
        "    await _initializeUI();\n" + MODULE.MAIN_LAYOUT +
        "}\n")
    (path / "js/ui/extensionSystem.js").write_text(
        "import * as Main from './main.js';\n"
        "        return this._extensions.get(uuid);\n"
        "            const uuid = info.get_name();\n"
        "            const existing = this.lookup(uuid);\n"
        "        return extensions.filter(item => !disabledExtensions.includes(item));\n")
    (path / "js/js-resources.gresource.xml").write_text(
        "<gresources>\n  <gresource prefix=\"/org/gnome/shell\">\n"
        "    <file>ui/main.js</file>\n"
        "  </gresource>\n</gresources>\n")


class NativeShellOverlayTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.source = Path(self.temp.name)
        self.overlay = ROOT / "gnome/nodalix-shell"
        fixture(self.source)

    def test_native_phone_link_is_bundled_and_legacy_settings_are_preserved(self):
        MODULE.prepare(self.source, self.overlay)
        main = (self.source / "js/ui/main.js").read_text()
        resources = (self.source / "js/js-resources.gresource.xml").read_text()
        extensions = (self.source / "js/ui/extensionSystem.js").read_text()
        self.assertIn("Nodalix.start({panel, sessionMode, extensionManager, notifyError})", main)
        self.assertLess(main.index("global.connect('shutdown', Nodalix.stop)"),
                        main.index("layoutManager = new Layout.LayoutManager()"))
        self.assertIn("nodalix/main.js", resources)
        self.assertIn("nodalix/connect.js", resources)
        self.assertIn("nodalix-connect@getnodalia.com", (self.source / "js/nodalix/modules.js").read_text())
        self.assertIn("!NodalixModules.managesModule(item)", extensions)
        self.assertIn("NodalixModules.lookupModule(uuid)", extensions)
        self.assertNotIn("set_strv", extensions)
        self.assertIn("bus_watch_name", (self.source / "js/nodalix/connect.js").read_text())
        self.assertIn("sessionMode.isGreeter", (self.source / "js/nodalix/main.js").read_text())

    def test_bundled_desktop_modules_keep_their_upstream_ids(self):
        MODULE.prepare(self.source, self.overlay)
        manifest = self.source / "js/nodalix/modules.json"
        registry = json.loads(manifest.read_text())
        module_ids = registry["bundled"] + registry["desktopPackages"]
        self.assertEqual(len(set(module_ids)), len(module_ids))
        self.assertIn("blur-my-shell@aunetx", module_ids)
        self.assertIn("tiling-assistant@leleat-on-github", module_ids)
        self.assertIn("global-menu@unmade.space", module_ids)
        self.assertIn("hanabi-extension@jeffshee.github.io", registry["desktopPackages"])
        self.assertIn("glocalsend@donnybeelo.github.com", registry["desktopPackages"])
        self.assertIn("nowplaying@epogonii.github.io", registry["bundled"])
        for uuid in set(module_ids) - {
            "hanabi-extension@jeffshee.github.io", "nowplaying@epogonii.github.io",
            "nodalix-login-background@getnodalia.com",
        }:
            path = ROOT / "gnome/extensions" / uuid
            self.assertTrue((path / "extension.js").is_file())
            self.assertEqual(json.loads((path / "metadata.json").read_text())["uuid"], uuid)

    def test_shutdown_uses_native_teardown_and_regular_disable_restores_desktop(self):
        script = r"""
const fs = require('fs');
const assert = require('assert');
const source = fs.readFileSync(process.argv[1], 'utf8');
const definition = source.slice(source.indexOf('export class DesktopModules'))
    .replace('export class', 'return class');
const records = new Map();
const Component = new Function('records', 'ExtensionState', 'logError', definition)(
    records, {INACTIVE: 2}, error => { throw error; });
for (const stopped of [false, true]) {
    const calls = [];
    const manager = {
        stopped,
        extensionManager: {
            _unloadExtensionStylesheet: () => calls.push('style'),
            emit: () => calls.push('signal'),
        },
    };
    const record = {uuid: 'test', stateObj: {
        disable: () => calls.push('disable'),
        shutdown: () => calls.push('shutdown'),
    }};
    records.set(record.uuid, record);
    Component.prototype.disable.call(manager, record);
    assert.deepStrictEqual(calls, [stopped ? 'shutdown' : 'disable', 'style', 'signal']);
    assert.strictEqual(records.size, 0);
    assert.strictEqual(record.enabled, false);
}
"""
        subprocess.run(["node", "-e", script, str(self.overlay / "modules.js")], check=True)

    def test_unrecognized_upstream_version_fails_without_writing(self):
        fixture(self.source, "52.0")
        before = (self.source / "js/ui/main.js").read_text()
        with self.assertRaisesRegex(ValueError, "pinned"):
            MODULE.prepare(self.source, self.overlay)
        self.assertEqual((self.source / "js/ui/main.js").read_text(), before)
        self.assertFalse((self.source / "js/nodalix").exists())

    def test_upstream_anchor_drift_fails_without_writing(self):
        path = self.source / "js/ui/extensionSystem.js"
        path.write_text("changed upstream code\n")
        with self.assertRaisesRegex(ValueError, "anchor differs"):
            MODULE.prepare(self.source, self.overlay)
        self.assertFalse((self.source / "js/nodalix").exists())

    def test_overlay_cannot_be_applied_twice(self):
        MODULE.prepare(self.source, self.overlay)
        with self.assertRaises(ValueError):
            MODULE.prepare(self.source, self.overlay)

    def test_version_coupled_to_nodalix_release(self):
        version = (ROOT / "VERSION").read_text().strip()
        pkgbuild = (ROOT / "packaging/nodalix-shell-preview/PKGBUILD").read_text()
        self.assertIn(f"pkgver={version}\n", pkgbuild)
        self.assertIn("conflicts=('gnome-shell')", pkgbuild)
        self.assertNotIn("replaces=(", pkgbuild)
        components = (ROOT / "release/components.json").read_text()
        self.assertNotIn('"nodalix-shell-preview"', components)


if __name__ == "__main__":
    unittest.main()
