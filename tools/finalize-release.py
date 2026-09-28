#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()

    with path.open("rb") as stream:
        for chunk in iter(
            lambda: stream.read(
                1024 * 1024
            ),
            b"",
        ):
            digest.update(chunk)

    return digest.hexdigest()


def verify_component_assets(
    directory: Path,
    manifest: dict,
) -> None:
    for component in manifest[
        "components"
    ]:
        name = component["asset"]

        if Path(name).name != name:
            raise RuntimeError(
                f"Unsafe component asset: {name}"
            )

        path = directory / name

        if not path.is_file():
            raise RuntimeError(
                f"Missing component asset: {name}"
            )

        if (
            path.stat().st_size
            != component["size"]
        ):
            raise RuntimeError(
                f"Size mismatch: {name}"
            )

        digest = sha256(path)

        if digest != component["sha256"]:
            raise RuntimeError(
                f"SHA-256 mismatch: {name}"
            )


def attach_iso(
    directory: Path,
    manifest: dict,
) -> None:
    images = sorted(
        directory.glob(
            "nodalix-*.iso"
        )
    )

    if not images:
        manifest.pop(
            "iso",
            None,
        )
        return

    if len(images) != 1:
        raise RuntimeError(
            "Expected exactly one ISO; found: "
            + ", ".join(
                image.name
                for image in images
            )
        )

    image = images[0]

    expected_name = (
        "nodalix-"
        f"{manifest['version']}"
        "-x86_64.iso"
    )

    if image.name != expected_name:
        raise RuntimeError(
            f"Expected {expected_name}, "
            f"found {image.name}"
        )

    digest = sha256(image)

    checksum_path = (
        directory
        / f"{image.name}.sha256"
    )

    if not checksum_path.is_file():
        raise RuntimeError(
            f"Missing {checksum_path.name}"
        )

    expected_checksum = (
        f"{digest}  {image.name}"
    )

    actual_checksum = (
        checksum_path
        .read_text(
            encoding="utf-8"
        )
        .strip()
    )

    if actual_checksum != expected_checksum:
        raise RuntimeError(
            "ISO checksum file does not "
            "match the generated ISO"
        )

    manifest["iso"] = {
        "asset": image.name,
        "sha256": digest,
        "size": image.stat().st_size,
    }


def regenerate_checksums(
    directory: Path,
) -> None:
    entries: list[str] = []

    for path in sorted(
        directory.iterdir()
    ):
        if (
            not path.is_file()
            or path.name == "SHA256SUMS"
        ):
            continue

        entries.append(
            f"{sha256(path)}  {path.name}"
        )

    (
        directory
        / "SHA256SUMS"
    ).write_text(
        "\n".join(entries)
        + "\n",
        encoding="utf-8",
    )


def main() -> None:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--assets-dir",
        type=Path,
        required=True,
    )

    args = parser.parse_args()

    directory = (
        args.assets_dir
        .resolve()
    )

    manifest_path = (
        directory
        / "nodalix-manifest.json"
    )

    manifest = json.loads(
        manifest_path.read_text(
            encoding="utf-8"
        )
    )

    verify_component_assets(
        directory,
        manifest,
    )

    attach_iso(
        directory,
        manifest,
    )

    manifest_path.write_text(
        json.dumps(
            manifest,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    regenerate_checksums(
        directory,
    )

    print(
        f"Finalized Nodalix OS "
        f"{manifest['version']}"
    )

    if "iso" in manifest:
        print(
            f"ISO: "
            f"{manifest['iso']['asset']}"
        )


if __name__ == "__main__":
    main()
