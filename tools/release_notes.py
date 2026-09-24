#!/usr/bin/env python3
"""Validate curated Nodalix release notes and build the final GitHub changelog."""

from __future__ import annotations

import argparse
import functools
import os
import re
import subprocess
import sys
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS_DIR))

from versioning import (
    canonical_version,
    compare_versions,
    is_stable,
    parse_version,
)

PLACEHOLDER_RE = re.compile(
    r"\b(TODO|TBD|PLACEHOLDER|WRITE ME|COMING SOON)\b",
    re.IGNORECASE,
)


def fail(message: str) -> None:
    print(f"::error::{message}", file=sys.stderr)
    raise SystemExit(message)


def release_notes_path(version: str) -> Path:
    version = canonical_version(version)
    parse_version(version)
    return Path("docs/releases") / f"v{version}.md"


def validate(version: str) -> tuple[Path, str]:
    version = canonical_version(version)
    path = release_notes_path(version)

    if not path.is_file():
        fail(
            f"Missing curated release notes for Nodalix {version}. "
            f"Create {path} with the user-facing highlights for this release."
        )

    text = path.read_text(encoding="utf-8").strip()

    if len(text) < 80:
        fail(
            f"{path} is too short. "
            "Add meaningful user-facing release highlights."
        )

    bullets = [
        line
        for line in text.splitlines()
        if line.lstrip().startswith("- ")
    ]

    if not bullets:
        fail(
            f"{path} must contain at least one Markdown bullet "
            "describing a user-facing change."
        )

    if PLACEHOLDER_RE.search(text):
        fail(f"{path} still contains TODO/TBD/placeholder text.")

    if re.search(r"^##\s+What(?:'|’)s changed", text, re.I | re.M):
        fail(
            f"Do not add \"What's changed\" to {path}; "
            "the release workflow generates it automatically."
        )

    if "Full changelog" in text:
        fail(
            f"Do not add \"Full changelog\" to {path}; "
            "the release workflow generates it automatically."
        )

    print(f"Validated curated release notes: {path}")
    return path, text


def git(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args],
        text=True,
    ).strip()


def is_ancestor(older: str, newer: str) -> bool:
    return (
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", older, newer],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    )


def previous_tag(current_tag: str) -> str | None:
    current_version = canonical_version(current_tag)
    current_is_stable = is_stable(current_version)

    candidates: list[str] = []

    for tag in git("tag", "--list").splitlines():
        tag = tag.strip()

        if not tag or tag == current_tag:
            continue

        try:
            candidate = canonical_version(tag)
            parse_version(candidate)
        except ValueError:
            continue

        if compare_versions(candidate, current_version) >= 0:
            continue

        # Stable releases compare against the previous stable release,
        # not against beta/RC builds of the same version.
        if current_is_stable and not is_stable(candidate):
            continue

        if not is_ancestor(tag, current_tag):
            continue

        candidates.append(tag)

    if not candidates:
        return None

    def compare(left: str, right: str) -> int:
        return compare_versions(
            canonical_version(left),
            canonical_version(right),
        )

    return max(
        candidates,
        key=functools.cmp_to_key(compare),
    )


def commit_lines(
    base: str | None,
    tag: str,
    repository_url: str,
) -> list[str]:
    revision = f"{base}..{tag}" if base else tag

    log = git(
        "log",
        "--reverse",
        "--format=%H%x09%s",
        revision,
    )

    lines: list[str] = []

    for row in log.splitlines():
        if not row.strip():
            continue

        sha, subject = row.split("\t", 1)
        short = sha[:7]

        lines.append(
            f"- {subject} "
            f"([{short}]({repository_url}/commit/{sha}))"
        )

    return lines


def build(tag: str, output: Path) -> None:
    version = canonical_version(tag)
    _, curated = validate(version)

    repository = os.environ.get(
        "GITHUB_REPOSITORY",
        "danielmigueltejedor/nodalix-os",
    )
    repository_url = f"https://github.com/{repository}"

    base = previous_tag(tag)
    changes = commit_lines(base, tag, repository_url)

    sections = [
        curated,
        "## What's changed\n\n"
        + (
            "\n".join(changes)
            if changes
            else "- No commits found."
        ),
    ]

    if base:
        sections.append(
            f"**Full changelog**: "
            f"[{base}...{tag}]"
            f"({repository_url}/compare/{base}...{tag})"
        )

    sections.append(
        "---\n\n"
        "*Release highlights are curated manually. "
        "The commit list and comparison link are generated automatically.*"
    )

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        "\n\n".join(sections) + "\n",
        encoding="utf-8",
    )

    print(f"Generated release notes: {output}")
    print(f"Previous release: {base or 'none'}")


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(
        dest="command",
        required=True,
    )

    validate_parser = commands.add_parser("validate")
    validate_parser.add_argument("--version", required=True)

    build_parser = commands.add_parser("build")
    build_parser.add_argument("--tag", required=True)
    build_parser.add_argument("--output", required=True)

    args = parser.parse_args()

    if args.command == "validate":
        validate(args.version)
    else:
        build(args.tag, Path(args.output))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
