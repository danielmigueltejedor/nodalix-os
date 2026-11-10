"""Keep NXPM tests independent of host package-manager lock files."""

from __future__ import annotations

import pytest


@pytest.fixture(autouse=True)
def isolate_package_manager_locks(monkeypatch, tmp_path_factory):
    root = tmp_path_factory.mktemp("nxpm-locks")
    monkeypatch.setenv("NXPM_UPDATER_LOCK", str(root / "update.lock"))
    monkeypatch.setenv("NXPM_PACMAN_LOCK", str(root / "db.lck"))
