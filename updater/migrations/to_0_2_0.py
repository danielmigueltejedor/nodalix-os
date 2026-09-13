"""Remove unowned 0.1.1 bootstrap files before 0.2.x packages claim them."""

from __future__ import annotations

from pathlib import Path
from typing import Callable

# Files created outside pacman during 0.1.1 bootstrap/stabilisation.
# Do not delete the parent directories — only these exact paths.
LEGACY_UNOWNED_FILES = (
    "/etc/nodalix-release",
    "/etc/systemd/user/default.target.wants/nodalix-shell.service",
    "/etc/systemd/system/timers.target.wants/nodalix-update-check.timer",
    "/usr/lib/qt6/qml/Caelestia/Blobs/caelestia-blobs.qmltypes",
    "/usr/lib/qt6/qml/Caelestia/Blobs/libcaelestia-blobs.so",
    "/usr/lib/qt6/qml/Caelestia/Blobs/libcaelestia-blobsplugin.so",
    "/usr/lib/qt6/qml/Caelestia/Blobs/qmldir",
)

# First 0.2 packaging that owns the paths above.
FIRST_PACKAGED_VERSION = "0.2.0-beta.1"


def has_unowned_legacy_files(
    files: tuple[str, ...] = LEGACY_UNOWNED_FILES,
    owned_check: Callable[[Path], bool] | None = None,
) -> bool:
    """True when a known leftover exists and is not owned by any package."""
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


def applies_0_1_1_to_0_2_0(
    current: str,
    target: str,
    compare_versions,
    *,
    owned_check: Callable[[Path], bool] | None = None,
    files: tuple[str, ...] | None = None,
    **_: object,
) -> bool:
    """Run when installing 0.2.x *and* either the origin is pre-0.2 packaging
    or known unowned leftovers are still on the filesystem.

    A machine that reports 0.2.0-beta.1 after a partial/manual recovery must
    still be cleaned if the bootstrap files remain unowned.
    """
    if compare_versions(target, FIRST_PACKAGED_VERSION) < 0:
        return False
    if compare_versions(current, FIRST_PACKAGED_VERSION) < 0:
        return True
    return has_unowned_legacy_files(
        files=files or LEGACY_UNOWNED_FILES,
        owned_check=owned_check,
    )
