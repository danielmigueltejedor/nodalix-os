#!/usr/bin/env python3
"""Shared SemVer helpers for Nodalix packaging and the updater tests."""
from __future__ import annotations

import re

VERSION_PATTERN = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z.-]+)?$"
)
NUMERIC_IDENTIFIER = re.compile(r"^(0|[1-9]\d*)$")


def canonical_version(value: str) -> str:
    text = value.strip()
    if text.startswith(("v", "V")):
        text = text[1:]
    if "+" in text:
        text = text.split("+", 1)[0]
    return text


def parse_version(value: str) -> tuple[int, int, int, tuple[tuple[int, object], ...] | None]:
    text = canonical_version(value)
    match = VERSION_PATTERN.fullmatch(text)
    if not match:
        raise ValueError(f"invalid semantic version: {value}")
    prerelease = match.group(4)
    identifiers = None
    if prerelease is not None:
        parts = []
        for part in prerelease.split("."):
            if part == "":
                raise ValueError(f"invalid semantic version: {value}")
            if NUMERIC_IDENTIFIER.fullmatch(part):
                parts.append((0, int(part)))
            elif part.isdigit():
                raise ValueError(f"invalid semantic version: {value}")
            else:
                parts.append((1, part))
        identifiers = tuple(parts)
    return int(match.group(1)), int(match.group(2)), int(match.group(3)), identifiers


def compare_versions(left: str, right: str) -> int:
    left_value = parse_version(left)
    right_value = parse_version(right)
    if left_value[:3] != right_value[:3]:
        return (left_value[:3] > right_value[:3]) - (left_value[:3] < right_value[:3])
    left_pre, right_pre = left_value[3], right_value[3]
    if left_pre is None or right_pre is None:
        return (left_pre is None) - (right_pre is None)
    for left_part, right_part in zip(left_pre, right_pre):
        if left_part != right_part:
            return (left_part > right_part) - (left_part < right_part)
    return (len(left_pre) > len(right_pre)) - (len(left_pre) < len(right_pre))


def is_prerelease(value: str) -> bool:
    return parse_version(value)[3] is not None


def is_stable(value: str) -> bool:
    return not is_prerelease(value)


def to_pkgver(value: str) -> str:
    """Arch pkgver cannot contain hyphens; SemVer prereleases map by removing them."""
    return canonical_version(value).replace("-", "")


def channel_for_version(value: str) -> str:
    return "beta" if is_prerelease(value) else "stable"


def is_prerelease_tag(tag: str) -> bool:
    text = canonical_version(tag)
    return bool(re.search(r"-(beta|rc)(\.|$)", text, flags=re.IGNORECASE))
