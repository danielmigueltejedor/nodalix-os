"""Writable state that belongs to the user, never to an installed application."""

from __future__ import annotations

import os
from pathlib import Path


def state_dir() -> Path:
    base = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local" / "state"))
    path = base / "nxpm"
    path.mkdir(parents=True, exist_ok=True)
    return path
