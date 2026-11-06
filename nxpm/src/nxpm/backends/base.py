"""Backend contract used by the engine and the tests."""

from __future__ import annotations

from typing import Protocol

from nxpm.model import Package


class Backend(Protocol):
    source: str

    def search(self, query: str) -> list[Package]:
        ...

    def list_installed(self) -> list[Package]:
        ...

    def list_updates(self) -> list[Package]:
        ...

    def details(self, package_id: str) -> Package:
        ...
