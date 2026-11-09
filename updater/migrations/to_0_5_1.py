"""Clear unowned release public keys so the updater package can own the path."""

from __future__ import annotations

from pathlib import Path
from typing import Callable

RELEASE_KEY_FILES = (
    "/usr/share/nodalix/keys/release.pub",
    "/usr/share/nodalix/keys/release.pub.pre-0.5.0",
)

FIRST_KEY_OWNER_VERSION = "0.5.0"


def has_unowned_release_keys(
    files: tuple[str, ...] = RELEASE_KEY_FILES,
    owned_check: Callable[[Path], bool] | None = None,
) -> bool:
    check = owned_check
    if check is None:
        from .registry import file_owned_by_package

        check = file_owned_by_package
    for raw in files:
        path = Path(raw)
        if not path.exists() and not path.is_symlink():
            continue
        if not check(path):
            return True
    return False


def applies_release_key_cleanup(
    current: str,
    target: str,
    compare_versions,
    *,
    owned_check: Callable[[Path], bool] | None = None,
    files: tuple[str, ...] | None = None,
    **_: object,
) -> bool:
    """Run when installing a release that ships the key and leftovers remain unowned."""
    if compare_versions(target, FIRST_KEY_OWNER_VERSION) < 0:
        return False
    return has_unowned_release_keys(
        files=files or RELEASE_KEY_FILES,
        owned_check=owned_check,
    )
