"""Remove unowned 0.1.1 bootstrap files before 0.2.x packages claim them."""

from __future__ import annotations

from pathlib import Path

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

# First 0.2 packaging that owns the paths above. 0.2.0-beta.1 already
# installed them via packages, so this must not run on 0.2.x systems.
FIRST_PACKAGED_VERSION = "0.2.0-beta.1"


def applies_0_1_1_to_0_2_0(current: str, target: str, compare_versions) -> bool:
    return (
        compare_versions(current, FIRST_PACKAGED_VERSION) < 0
        and compare_versions(target, FIRST_PACKAGED_VERSION) >= 0
    )
