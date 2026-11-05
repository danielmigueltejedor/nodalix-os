#!/usr/bin/env python3
"""Apply the Nodalix native-shell overlay to the exact GNOME Shell 51.1 source.

Fail closed when upstream changes: never patch an unknown GNOME Shell release.
This prepares source only; it never touches the installed desktop.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import sys

GNOME_VERSION = "51.1"
RESOURCES = ("main.js", "connect.js", "modules.js", "modules.json")

MAIN_IMPORT = "import * as Panel from './panel.js';\n"
MAIN_INITIALIZE = "    await _initializeUI();\n"
MAIN_LAYOUT = "    layoutManager = new Layout.LayoutManager();\n"
RESOURCE_ANCHOR = "    <file>ui/main.js</file>\n"
EXTENSION_IMPORT = "import * as Main from './main.js';\n"
EXTENSION_LOOKUP = "        return this._extensions.get(uuid);\n"
EXTENSION_DISCOVERY = "            const uuid = info.get_name();\n            const existing = this.lookup(uuid);\n"
GETTEXT_DETECTION = """        const extensionLine = stack.find(
            line => line.includes(basePath));"""
EXTENSION_FILTER = "        return extensions.filter(item => !disabledExtensions.includes(item));\n"


def replace_once(path: Path, old: str, new: str) -> None:
    content = path.read_text(encoding="utf-8")
    if content.count(old) != 1:
        raise ValueError(f"{path}: expected exactly one known GNOME 51.1 anchor")
    path.write_text(content.replace(old, new, 1), encoding="utf-8")


def prepare(source: Path, overlay: Path) -> None:
    source = source.resolve()
    overlay = overlay.resolve()
    meson = (source / "meson.build").read_text(encoding="utf-8")
    if f"version: '{GNOME_VERSION}'" not in meson or "mutter_api_version = '51'" not in meson:
        raise ValueError("Nodalix Shell requires the explicitly pinned GNOME Shell 51.1 / Mutter 51 sources")

    main = source / "js/ui/main.js"
    resources = source / "js/js-resources.gresource.xml"
    extensions = source / "js/ui/extensionSystem.js"
    gettext = source / "js/extensions/sharedInternals.js"
    for path, needle in (
        (main, MAIN_IMPORT),
        (main, MAIN_INITIALIZE),
        (main, MAIN_LAYOUT),
        (resources, RESOURCE_ANCHOR),
        (extensions, EXTENSION_IMPORT),
        (extensions, EXTENSION_LOOKUP),
        (extensions, EXTENSION_DISCOVERY),
        (extensions, EXTENSION_FILTER),
        (gettext, GETTEXT_DETECTION),
        (gettext, "extensionLine.indexOf(basePath)"),
    ):
        if path.read_text(encoding="utf-8").count(needle) != 1:
            raise ValueError(f"{path}: upstream anchor differs; port and test before building")

    for name in RESOURCES:
        if not (overlay / name).is_file():
            raise FileNotFoundError(overlay / name)
        if (source / "js/nodalix" / name).exists():
            raise ValueError(f"native module already exists: {name}")

    destination = source / "js/nodalix"
    destination.mkdir(parents=True, exist_ok=True)
    for name in RESOURCES:
        shutil.copyfile(overlay / name, destination / name)

    replace_once(
        main, MAIN_IMPORT,
        MAIN_IMPORT + "import * as Nodalix from '../nodalix/main.js';\n")
    replace_once(
        main, MAIN_INITIALIZE,
        MAIN_INITIALIZE + "\n    // First-party Nodalix modules are initialized after GNOME UI.\n"
        + "    Nodalix.start({panel, sessionMode, extensionManager, notifyError});\n")
    # Connect before LayoutManager registers its actor-destruction handler.
    replace_once(
        main, MAIN_LAYOUT,
        "    global.connect('shutdown', Nodalix.stop);\n" + MAIN_LAYOUT)
    replace_once(
        resources, RESOURCE_ANCHOR,
        RESOURCE_ANCHOR + "".join(f"    <file>nodalix/{name}</file>\n" for name in RESOURCES))
    replace_once(
        extensions, EXTENSION_IMPORT,
        EXTENSION_IMPORT + "import * as NodalixModules from '../nodalix/modules.js';\n")
    replace_once(
        extensions, EXTENSION_LOOKUP,
        "        return this._extensions.get(uuid) ?? NodalixModules.lookupModule(uuid);\n")
    replace_once(
        extensions, EXTENSION_DISCOVERY,
        "            const uuid = info.get_name();\n"
        + "            if (NodalixModules.managesModule(uuid))\n"
        + "                return null;\n"
        + "            const existing = this.lookup(uuid);\n")
    # Skip the old extension without changing the user's GNOME settings. The
    # standard Arch GNOME Shell will honor it again after reverting the package.
    replace_once(
        extensions, EXTENSION_FILTER,
        "        return extensions.filter(item => !disabledExtensions.includes(item) &&\n"
        + "            !NodalixModules.managesModule(item));\n")
    replace_once(
        gettext, GETTEXT_DETECTION,
        "        const paths = [basePath, '/gnome-shell/nodalix-modules/', '/gnome-shell/nodalix-runtime/'];\n"
        "        const extensionLine = stack.find(line => paths.some(path => line.includes(path)));")
    replace_once(
        gettext, "extensionLine.indexOf(basePath)",
        "extensionLine.indexOf(paths.find(base => extensionLine.includes(base)))")
    print(f"Nodalix Shell native overlay applied to GNOME Shell {GNOME_VERSION}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Unpacked upstream GNOME Shell sources")
    parser.add_argument("--overlay", type=Path, default=Path(__file__).resolve().parents[1] / "gnome/nodalix-shell")
    args = parser.parse_args()
    try:
        prepare(args.source, args.overlay)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
