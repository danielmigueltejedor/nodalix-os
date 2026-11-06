"""Operations shared by the CLI and the D-Bus service."""

from __future__ import annotations

import logging
import os
import subprocess
import sys

from nxpm.config import Config, load_config
from nxpm.errors import NxpmError
from nxpm.grouping import group_packages, preferred_only
from nxpm.model import Package, parse_package_id, validate_query
from nxpm.transactions import Transaction, TransactionManager

log = logging.getLogger("nxpm")

# Root-owned programs that Polkit authorizes by path. pkexec never runs the
# user's interpreter or a module found through the user's environment.
HELPER_DIR = "/usr/lib/nodalix-nxpm"


class Engine:
    def __init__(self, config: Config, backends: dict, transactions: TransactionManager | None = None) -> None:
        self.config = config
        self.backends = backends
        self.transactions = transactions or TransactionManager()

    def sources(self) -> list[dict]:
        rows = []
        for name, spec in self.config.sources.items():
            rows.append(
                {
                    "id": name,
                    "enabled": spec.enabled and spec.implemented,
                    "implemented": spec.implemented,
                    "reason": spec.reason,
                }
            )
        return rows

    def search(self, query: str) -> list[Package]:
        text = validate_query(query)
        found: list[Package] = []
        for name in self.config.active_sources():
            backend = self.backends.get(name)
            if backend is None:
                continue
            found.extend(backend.search(text))
        return preferred_only(group_packages(found, self.config.preference))

    def list_installed(self, source: str = "") -> list[Package]:
        packages: list[Package] = []
        for name in self._selected(source):
            packages.extend(self.backends[name].list_installed())
        return packages

    def list_updates(self) -> list[Package]:
        packages: list[Package] = []
        for name in self.config.active_sources():
            backend = self.backends.get(name)
            if backend is None:
                continue
            packages.extend(backend.list_updates())
        return group_packages(packages, self.config.preference)

    def details(self, package_id: str) -> Package:
        source, _payload = parse_package_id(package_id)
        backend = self._backend(source)
        return backend.details(package_id)

    def install(self, package_id: str, source: str) -> Transaction:
        return self._change("install", package_id, source)

    def remove(self, package_id: str, source: str) -> Transaction:
        return self._change("remove", package_id, source)

    def update(self, package_id: str) -> Transaction:
        source, _payload = parse_package_id(package_id)
        if source == "pacman":
            raise NxpmError(
                "unsupported",
                "Official repository updates are applied together. Use a full system update.",
            )
        return self._change("update", package_id, source)

    def upgrade_system(self) -> Transaction:
        if "pacman" not in self.backends or not self.config.sources["pacman"].enabled:
            raise NxpmError("disabled", "Official repository updates are disabled.")

        def runner(transaction: Transaction, manager: TransactionManager) -> None:
            if os.geteuid() == 0:
                self.backends["pacman"].upgrade_system(lambda progress, message: self._progress(transaction, manager, progress, message))
                return
            self._run_privileged(transaction, manager, ("upgrade-system", "privileged-upgrade"), "System update finished")

        return self.transactions.submit("upgrade-system", "pacman:*", "pacman", runner)

    def _remove_system_package(self, package_id: str) -> Transaction:
        if "pacman" not in self.backends or not self.config.sources["pacman"].enabled:
            raise NxpmError("disabled", "Official repository changes are disabled.")

        def runner(transaction: Transaction, manager: TransactionManager) -> None:
            if os.geteuid() == 0:
                self.backends["pacman"].remove_package(
                    package_id,
                    lambda progress, message: self._progress(transaction, manager, progress, message),
                )
                return
            self._run_privileged(transaction, manager, ("remove-package", "privileged-remove"), "Package removed", package_id)

        return self.transactions.submit("remove", package_id, "pacman", runner)

    def _run_privileged(
        self, transaction: Transaction, manager: TransactionManager, command: tuple[str, str], done: str, *arguments: str
    ) -> None:
        helper_name, cli_name = command
        helper = os.path.join(os.environ.get("NXPM_HELPER_DIR", HELPER_DIR), helper_name)
        if os.environ.get("NXPM_NO_PKEXEC") == "1":
            # Development and tests: the caller is already authorized.
            argv = [sys.executable, "-m", "nxpm", cli_name, *arguments]
        elif os.path.isfile(helper):
            argv = ["pkexec", helper, *arguments]
        else:
            raise NxpmError(
                "unsupported",
                "NXPM is not installed system-wide, so it cannot ask for administrator authorization.",
            )
        transaction.message = "Waiting for administrator authorization"
        manager.publish(transaction)
        completed = subprocess.run(argv, capture_output=True, text=True, check=False)
        if completed.returncode != 0:
            detail = (completed.stderr or completed.stdout or "Authorization was not granted.").strip()
            raise NxpmError("authorization-required", detail)
        transaction.message = done
        transaction.progress = 1.0

    def import_file(self, kind: str, path: str) -> Transaction:
        if kind not in {"appimage", "deb"}:
            raise NxpmError("invalid-input", "NXPM can import an AppImage or a Debian package.")
        backend = self._backend("appimage")

        def runner(transaction: Transaction, manager: TransactionManager) -> None:
            package = backend.import_file(
                kind,
                path,
                lambda progress, message: self._progress(transaction, manager, progress, message),
            )
            transaction.message = package.summary or package.id

        return self.transactions.submit("import", "appimage:import", "appimage", runner)

    def permissions(self, package_id: str) -> list[dict]:
        source, _payload = parse_package_id(package_id)
        if source != "flatpak":
            raise NxpmError("unsupported", "Permission control is available for Flatpak applications.")
        return self._backend("flatpak").permissions(package_id)

    def reset_permissions(self, package_id: str) -> list[dict]:
        source, _payload = parse_package_id(package_id)
        if source != "flatpak":
            raise NxpmError("unsupported", "Permission control is available for Flatpak applications.")
        return self._backend("flatpak").reset_permissions(package_id)

    def _change(self, operation: str, package_id: str, source: str) -> Transaction:
        parsed_source, _payload = parse_package_id(package_id)
        if source and source != parsed_source:
            raise NxpmError("invalid-input", "The requested source does not match the package.")
        if parsed_source == "pacman":
            if operation == "remove":
                return self._remove_system_package(package_id)
            raise NxpmError(
                "authorization-required",
                "Installing or updating one official package would leave the system partly updated. Use a full system update.",
            )
        if parsed_source not in {"flatpak", "appimage"}:
            raise NxpmError("unsupported", "That software source cannot be changed.")
        backend = self._backend(parsed_source)

        def runner(transaction: Transaction, manager: TransactionManager) -> None:
            backend.apply(
                package_id,
                operation,
                lambda progress, message: self._progress(transaction, manager, progress, message),
            )

        return self.transactions.submit(operation, package_id, parsed_source, runner)

    def _progress(self, transaction: Transaction, manager: TransactionManager, progress: float, message: str) -> None:
        if progress >= 0:
            transaction.progress = progress
        if message:
            transaction.message = message
        manager.publish(transaction)

    def _selected(self, source: str) -> list[str]:
        if not source:
            return self.config.active_sources()
        if source not in self.config.sources:
            raise NxpmError("invalid-input", "Unknown software source.")
        spec = self.config.sources[source]
        if not spec.implemented:
            raise NxpmError("disabled", spec.reason)
        if not spec.enabled:
            raise NxpmError("disabled", f"{source} is turned off.")
        if source not in self.backends:
            raise NxpmError("disabled", f"{source} is not available.")
        return [source]

    def _backend(self, source: str):
        self._selected(source)
        return self.backends[source]


def build_engine(config: Config | None = None) -> Engine:
    loaded = config or load_config()
    backends = {}
    if loaded.sources["flatpak"].enabled and loaded.sources["flatpak"].implemented:
        from nxpm.backends.flatpak import FlatpakBackend

        backends["flatpak"] = FlatpakBackend()
    if loaded.sources["pacman"].enabled and loaded.sources["pacman"].implemented:
        from nxpm.backends.pacman import PacmanBackend

        backends["pacman"] = PacmanBackend()
    if loaded.sources["appimage"].enabled and loaded.sources["appimage"].implemented:
        from nxpm.backends.appimage import AppImageBackend

        backends["appimage"] = AppImageBackend()
    for name, spec in loaded.sources.items():
        if spec.enabled and not spec.implemented:
            log.warning("%s was requested but stays disabled: %s", name, spec.reason)
    return Engine(loaded, backends)
