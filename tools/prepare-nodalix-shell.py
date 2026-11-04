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
NATIVE_EXTENSION = "nodalix-connect@getnodalia.com"

MAIN_IMPORT = "import * as Panel from './panel.js';\n"
MAIN_INITIALIZE = "    await _initializeUI();\n"
RESOURCE_ANCHOR = "    <file>ui/main.js</file>\n"
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
    for path, needle in (
        (main, MAIN_IMPORT),
        (main, MAIN_INITIALIZE),
        (resources, RESOURCE_ANCHOR),
        (extensions, EXTENSION_FILTER),
    ):
        if path.read_text(encoding="utf-8").count(needle) != 1:
            raise ValueError(f"{path}: upstream anchor differs; port and test before building")

    for name in ("main.js", "connect.js"):
        if not (overlay / name).is_file():
            raise FileNotFoundError(overlay / name)
        if (source / "js/nodalix" / name).exists():
            raise ValueError(f"native module already exists: {name}")

    destination = source / "js/nodalix"
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("main.js", "connect.js"):
        shutil.copyfile(overlay / name, destination / name)

    replace_once(
        main, MAIN_IMPORT,
        MAIN_IMPORT + "import * as Nodalix from '../nodalix/main.js';\n")
    replace_once(
        main, MAIN_INITIALIZE,
        MAIN_INITIALIZE + "\n    // First-party Nodalix modules are initialized after GNOME UI.\n"
        + "    Nodalix.start({panel, sessionMode, notifyError});\n")
    replace_once(
        resources, RESOURCE_ANCHOR,
        RESOURCE_ANCHOR + "    <file>nodalix/main.js</file>\n"
        + "    <file>nodalix/connect.js</file>\n")
    # Skip the old extension without changing the user's GNOME settings. The
    # standard Arch GNOME Shell will honor it again after reverting the package.
    replace_once(
        extensions, EXTENSION_FILTER,
        "        return extensions.filter(item => !disabledExtensions.includes(item) &&\n"
        + f"            item !== '{NATIVE_EXTENSION}');\n")
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
