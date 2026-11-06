"""Read official packages through pyalpm and apply changes only as root.

Partial upgrades are refused. A system update uses libalpm's full sysupgrade.
"""

from __future__ import annotations

import logging
import os
import re
from pathlib import Path

from nxpm.errors import NxpmError
from nxpm.model import Package, parse_package_id

log = logging.getLogger("nxpm.pacman")
_RESULT_LIMIT = 80


def open_handle(config_path: str = "/etc/pacman.conf"):
    from pycman.config import init_with_config

    return init_with_config(config_path)


def package_from_alpm(pkg, *, installed: bool, available: str = "", update: bool = False) -> Package:
    name = str(pkg.name)
    version = str(pkg.version)
    return Package(
        id=f"pacman:{name}",
        name=name,
        summary=str(getattr(pkg, "desc", "") or ""),
        version=version,
        available_version=available,
        source="pacman",
        kind="package",
        installed=installed,
        update=update,
        appstream_id="",
        icon_name=name,
        size=int(getattr(pkg, "isize", 0) or getattr(pkg, "size", 0) or 0),
        risk="medium",
    )


class PacmanBackend:
    source = "pacman"

    def __init__(self, handle_factory=None, appstream_ids: dict[str, str] | None = None) -> None:
        self._handle_factory = handle_factory or open_handle
        self._appstream_ids = appstream_ids
        self._handle = None

    def handle(self):
        if self._handle is None:
            self._handle = self._handle_factory()
        return self._handle

    def search(self, query: str) -> list[Package]:
        handle = self.handle()
        expression = re.escape(query)
        found: dict[str, Package] = {}
        try:
            local_matches = handle.get_localdb().search(expression)
        except Exception as exc:
            log.warning("Pacman local search failed: %s", exc)
            local_matches = []
        for pkg in local_matches:
            package = self._decorate(package_from_alpm(pkg, installed=True))
            found[package.id] = package
        for database in self._syncdbs(handle):
            try:
                matches = database.search(expression)
            except Exception as exc:
                log.warning("Pacman search failed for %s: %s", database.name, exc)
                continue
            for pkg in matches:
                package = self._decorate(package_from_alpm(pkg, installed=package_id_installed(handle, pkg.name)))
                current = found.get(package.id)
                if current is None:
                    found[package.id] = package
                elif not current.installed:
                    found[package.id] = package
                if len(found) >= _RESULT_LIMIT:
                    return list(found.values())
        return list(found.values())

    def list_installed(self) -> list[Package]:
        packages = []
        for pkg in self.handle().get_localdb().pkgcache:
            packages.append(self._decorate(package_from_alpm(pkg, installed=True)))
        return packages

    def list_updates(self) -> list[Package]:
        import pyalpm

        handle = self.handle()
        ignored = {str(name) for name in (handle.ignorepkgs or [])}
        sync = {}
        for database in self._syncdbs(handle):
            for pkg in database.pkgcache:
                current = sync.get(pkg.name)
                if current is None or pyalpm.vercmp(str(pkg.version), str(current.version)) > 0:
                    sync[pkg.name] = pkg

        updates = []
        for pkg in handle.get_localdb().pkgcache:
            if pkg.name in ignored:
                continue
            candidate = sync.get(pkg.name)
            if candidate is None:
                continue
            if pyalpm.vercmp(str(candidate.version), str(pkg.version)) > 0:
                updates.append(
                    self._decorate(
                        package_from_alpm(
                            pkg,
                            installed=True,
                            available=str(candidate.version),
                            update=True,
                        )
                    )
                )
        return updates

    def details(self, package_id: str) -> Package:
        source, name = parse_package_id(package_id)
        if source != "pacman":
            raise NxpmError("invalid-input", "That identifier is not an official package.")
        handle = self.handle()
        local = handle.get_localdb().get_pkg(name)
        if local is not None:
            available = ""
            update = False
            for item in self.list_updates():
                if item.id == package_id:
                    available = item.available_version
                    update = True
                    break
            return self._decorate(package_from_alpm(local, installed=True, available=available, update=update))
        for database in self._syncdbs(handle):
            pkg = database.get_pkg(name)
            if pkg is not None:
                return self._decorate(package_from_alpm(pkg, installed=False))
        raise NxpmError("not-found", "The official repositories do not contain that package.")

    def upgrade_system(self, progress) -> None:
        if os.geteuid() != 0:
            raise NxpmError(
                "authorization-required",
                "Updating official packages needs administrator authorization.",
            )
        handle = self.handle()
        self._write_inventory(handle)
        progress(0.1, "Preparing a full system update")
        transaction = handle.init_transaction()
        try:
            transaction.sysupgrade(False)
            progress(0.4, "Resolving official packages")
            transaction.prepare()
            progress(0.7, "Applying the system update")
            transaction.commit()
        finally:
            transaction.release()
        progress(1.0, "System update finished")

    def remove_package(self, package_id: str, progress) -> None:
        """Remove one installed package. libalpm refuses when something still needs it."""

        if os.geteuid() != 0:
            raise NxpmError(
                "authorization-required",
                "Removing official packages needs administrator authorization.",
            )
        source, name = parse_package_id(package_id)
        if source != "pacman":
            raise NxpmError("invalid-input", "That identifier is not an official package.")
        handle = self.handle()
        local = handle.get_localdb().get_pkg(name)
        if local is None:
            raise NxpmError("not-found", "That package is not installed.")
        self._write_inventory(handle)
        progress(0.2, "Checking what depends on this package")
        transaction = handle.init_transaction()
        try:
            transaction.remove_pkg(local)
            try:
                transaction.prepare()
            except Exception as exc:
                raise NxpmError(
                    "failed",
                    f"The package cannot be removed without breaking others: {exc}",
                ) from exc
            progress(0.6, "Removing the package")
            transaction.commit()
        finally:
            transaction.release()
        progress(1.0, "Package removed")

    def _decorate(self, package: Package) -> Package:
        package.appstream_id = self._ids().get(package.name, "")
        if package.appstream_id:
            package.icon_name = package.appstream_id
        return package

    def _ids(self) -> dict[str, str]:
        if self._appstream_ids is not None:
            return self._appstream_ids
        self._appstream_ids = load_appstream_package_ids()
        return self._appstream_ids

    def _syncdbs(self, handle):
        try:
            return list(handle.get_syncdbs() or [])
        except Exception as exc:
            log.warning("Pacman sync databases are unavailable: %s", exc)
            return []

    def _write_inventory(self, handle) -> None:
        from nxpm.paths import state_dir

        lines = [f"{pkg.name} {pkg.version}" for pkg in handle.get_localdb().pkgcache]
        path = state_dir() / "pacman-inventory.txt"
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def package_id_installed(handle, name: str) -> bool:
    try:
        return handle.get_localdb().get_pkg(name) is not None
    except Exception:
        return False


def load_appstream_package_ids() -> dict[str, str]:
    try:
        import gi

        gi.require_version("AppStream", "1.0")
        from gi.repository import AppStream
    except Exception as exc:
        log.info("AppStream is not available for package grouping: %s", exc)
        return {}
    mapping: dict[str, str] = {}
    try:
        pool = AppStream.Pool()
        pool.load()
        components = pool.get_components()
        count = int(components.get_size())
    except Exception as exc:
        log.info("AppStream pool could not be loaded: %s", exc)
        return {}
    for index in range(count):
        try:
            component = components.index_safe(index)
            pkgname = component.get_pkgname()
            component_id = component.get_id()
        except Exception:
            continue
        if not pkgname or not component_id:
            continue
        text = str(component_id)
        if text.endswith(".desktop"):
            text = text[: -len(".desktop")]
        mapping.setdefault(str(pkgname), text)
    return mapping


def inventory_path() -> Path:
    from nxpm.paths import state_dir

    return state_dir() / "pacman-inventory.txt"
