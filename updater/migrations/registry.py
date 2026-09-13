"""Run version-gated migrations before pacman -U."""

from __future__ import annotations

import json
import os
import shutil
import stat
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from .to_0_2_0 import LEGACY_UNOWNED_FILES, applies_0_1_1_to_0_2_0

MAX_BACKUP_BYTES = 32 * 1024 * 1024


class MigrationError(RuntimeError):
    code = "migration_failed"
    status = "migration_failed"
    recoverable = False


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _safe_relpath(path: Path) -> Path:
    relative = Path(os.path.relpath(path, "/"))
    if relative.is_absolute() or ".." in relative.parts:
        raise MigrationError(f"refusing to backup unsafe path: {path}")
    return relative


def file_owned_by_package(
    path: Path,
    pacman: Callable[..., object] | None = None,
) -> bool:
    runner = pacman
    if runner is None:
        import subprocess

        runner = subprocess.run
    result = runner(
        ["pacman", "-Qo", str(path)],
        capture_output=True,
        text=True,
        check=False,
    )
    return int(getattr(result, "returncode", 1)) == 0


def _copy_legacy(source: Path, destination: Path) -> dict:
    destination.parent.mkdir(parents=True, exist_ok=True)
    record: dict = {
        "path": str(source),
        "backup": str(destination),
        "mtime": None,
        "mode": None,
        "type": "file",
    }
    try:
        info = source.lstat()
        record["mtime"] = datetime.fromtimestamp(info.st_mtime, timezone.utc).isoformat()
        record["mode"] = stat.filemode(info.st_mode)
    except OSError:
        pass
    if source.is_symlink():
        target = os.readlink(source)
        destination.symlink_to(target)
        record["type"] = "symlink"
        record["target"] = target
        return record
    size = source.stat().st_size
    record["size"] = size
    if size > MAX_BACKUP_BYTES:
        record["type"] = "skipped-content"
        record["reason"] = f"file larger than {MAX_BACKUP_BYTES} bytes"
        destination.write_text("", encoding="utf-8")
        return record
    shutil.copy2(source, destination, follow_symlinks=False)
    record["type"] = "file"
    return record


def remove_unowned_legacy_files(
    *,
    files: tuple[str, ...] = LEGACY_UNOWNED_FILES,
    backup_dir: Path,
    owned_check: Callable[[Path], bool] | None = None,
    log: Callable[[str], None] | None = None,
) -> dict:
    """Idempotent cleanup of known unowned 0.1.1 leftovers.

    Owned files are left untouched. Missing files are a no-op.
    """
    backup_dir.mkdir(parents=True, exist_ok=True)
    log_path = backup_dir / "migration.log"
    manifest_path = backup_dir / "manifest.json"
    files_root = backup_dir / "files"
    check = owned_check or (lambda path: file_owned_by_package(path))
    lines: list[str] = []
    removed: list[dict] = []
    skipped: list[dict] = []

    def write_log(message: str) -> None:
        entry = f"{utc_now()} {message}"
        lines.append(entry)
        if log is not None:
            log(message)

    for raw in files:
        path = Path(raw)
        if not path.exists() and not path.is_symlink():
            skipped.append({"path": str(path), "reason": "missing"})
            write_log(f"skipped {path} (missing)")
            continue
        if check(path):
            skipped.append({"path": str(path), "reason": "owned"})
            write_log(f"skipped {path} (owned by a package)")
            continue
        backup = files_root / _safe_relpath(path)
        record = _copy_legacy(path, backup)
        path.unlink()
        record["action"] = "removed"
        removed.append(record)
        write_log(f"removed {path} (unowned)")

    payload = {
        "id": "0.1.1-to-0.2.0",
        "timestamp": utc_now(),
        "files": list(files),
        "removed": removed,
        "skipped": skipped,
    }
    manifest_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    log_path.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
    return payload


def run_migrations(
    current_version: str,
    target_version: str,
    *,
    compare_versions: Callable[[str, str], int],
    state_dir: Path,
    owned_check: Callable[[Path], bool] | None = None,
    log: Callable[[str], None] | None = None,
) -> list[dict]:
    """Execute migrations whose version gate matches [current, target]."""
    results: list[dict] = []
    migrations = [
        {
            "id": "0.1.1-to-0.2.0",
            "applies": applies_0_1_1_to_0_2_0,
            "run": lambda: remove_unowned_legacy_files(
                backup_dir=state_dir / "migrations" / "0.1.1-to-0.2.0",
                owned_check=owned_check,
                log=log,
            ),
        }
    ]
    for migration in migrations:
        if not migration["applies"](current_version, target_version, compare_versions):
            if log is not None:
                log(f"skip migration {migration['id']} ({current_version} -> {target_version})")
            continue
        if log is not None:
            log(f"run migration {migration['id']} ({current_version} -> {target_version})")
        try:
            result = migration["run"]()
        except MigrationError:
            raise
        except OSError as error:
            raise MigrationError(f"{migration['id']} failed: {error}") from error
        result = dict(result or {})
        result["id"] = migration["id"]
        results.append(result)
    return results
