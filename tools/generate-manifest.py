#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT / "tools") not in sys.path:
    sys.path.insert(0, str(ROOT / "tools"))

from versioning import canonical_version, to_pkgver, VERSION_PATTERN  # noqa: E402


SHA256_RE = __import__("re").compile(r"^[0-9a-f]{64}$")


class ManifestError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def version(path: Path) -> str:
    value = canonical_version(path.read_text(encoding="utf-8"))
    if not VERSION_PATTERN.fullmatch(value):
        raise ManifestError(f"invalid canonical version: {value!r}")
    return value


def unique_package_asset(directory: Path, package: str, component_version: str) -> Path:
    pkgver = to_pkgver(component_version)
    matches = [
        path
        for path in sorted(directory.glob(f"{package}-{pkgver}-*.pkg.tar.zst"))
        if "-debug-" not in path.name and not path.name.endswith(".sig")
    ]
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
        component_version = canonical_version(str(component["version"]))
        if not VERSION_PATTERN.fullmatch(component_version):
            raise ManifestError(f"invalid component version: {component['version']}")
        asset = unique_package_asset(assets_dir, component["package"], component_version)
        digest = sha256(asset)
        if not SHA256_RE.fullmatch(digest):
            raise ManifestError(f"invalid SHA-256 for {component['id']}")
        component.update(
            version=component_version,
            asset=asset.name,
            sha256=digest,
            size=asset.stat().st_size,
        )
        components.append(component)
        seen_ids.add(component["id"])
        seen_packages.add(component["package"])

    if not components:
        raise ManifestError("release has no components")
    if not any(component["id"] == "nodalix-release" and component["required"] for component in components):
        raise ManifestError("nodalix-release must be required")

    channel = definition.get("channel", "stable")
    if channel not in ("stable", "beta"):
        raise ManifestError("channel must be stable or beta")

    return {
        # `schema` and the aggregate restart flags keep the 0.1.0 updater able
        # to bootstrap into 0.2.x. New clients validate `schema_version` and
        # each component's restart policy.
        "schema": 1,
        "schema_version": 1,
        "version": release_version,
        "arch": definition.get("arch", "x86_64"),
        "channel": channel,
        "minimum_version": definition["minimum_version"],
        "components": components,
        "shell_restart_required": any(item["restart"] == "shell" for item in components),
        "reboot_required": any(item["restart"] == "system" for item in components),
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
        tag = canonical_version(os_environ_tag())
        if tag and tag != manifest["version"]:
            raise ManifestError(f"manifest {manifest['version']} does not match tag {tag}")
    except (OSError, ValueError, KeyError, ManifestError) as error:
        parser.error(str(error))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


def os_environ_tag() -> str:
    import os

    return os.environ.get("NODALIX_RELEASE_TAG", "")


if __name__ == "__main__":
    raise SystemExit(main())
