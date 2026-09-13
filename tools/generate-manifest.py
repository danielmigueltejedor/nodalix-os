#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class ManifestError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def version(path: Path) -> str:
    value = path.read_text(encoding="utf-8").strip()
    if not VERSION_RE.fullmatch(value):
        raise ManifestError(f"invalid canonical version: {value!r}")
    return value


def unique_package_asset(directory: Path, package: str, component_version: str) -> Path:
    matches = sorted(directory.glob(f"{package}-{component_version}-*.pkg.tar.zst"))
    if len(matches) != 1:
        names = ", ".join(path.name for path in matches) or "none"
        raise ManifestError(
            f"expected one package for {package} {component_version}; found {names}"
        )
    return matches[0]


def generate(version_file: Path, definition_file: Path, assets_dir: Path) -> dict:
    release_version = version(version_file)
    definition = json.loads(definition_file.read_text(encoding="utf-8"))
    if definition.get("schema_version") != 1:
        raise ManifestError("component definition schema_version must be 1")

    components = []
    seen_ids: set[str] = set()
    seen_packages: set[str] = set()
    for source in definition.get("components", []):
        component = dict(source)
        for key in ("id", "name", "package", "version", "required", "restart"):
            if key not in component:
                raise ManifestError(f"component is missing {key}")
        if component["id"] in seen_ids or component["package"] in seen_packages:
            raise ManifestError(f"duplicate component: {component['id']}")
        if not VERSION_RE.fullmatch(component["version"]):
            raise ManifestError(f"invalid component version: {component['version']}")
        asset = unique_package_asset(assets_dir, component["package"], component["version"])
        component.update(asset=asset.name, sha256=sha256(asset), size=asset.stat().st_size)
        components.append(component)
        seen_ids.add(component["id"])
        seen_packages.add(component["package"])

    if not components:
        raise ManifestError("release has no components")
    if not any(component["id"] == "nodalix-release" and component["required"] for component in components):
        raise ManifestError("nodalix-release must be required")

    return {
        # `schema` and the aggregate restart flags keep the 0.1.0 updater able
        # to bootstrap into 0.2.0. New clients validate `schema_version` and
        # each component's restart policy.
        "schema": 1,
        "schema_version": 1,
        "version": release_version,
        "channel": definition.get("channel", "stable"),
        "minimum_version": definition["minimum_version"],
        "components": components,
        "shell_restart_required": any(c["restart"] == "shell" for c in components),
        "reboot_required": any(c["restart"] == "system" for c in components),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version-file", type=Path, default=Path("VERSION"))
    parser.add_argument("--components", type=Path, default=Path("release/components.json"))
    parser.add_argument("--assets-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        manifest = generate(args.version_file, args.components, args.assets_dir)
    except (OSError, ValueError, KeyError, ManifestError) as error:
        parser.error(str(error))
    args.output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
