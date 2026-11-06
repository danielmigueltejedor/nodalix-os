import json
from pathlib import Path

from nxpm.config import load_config
from nxpm.engine import Engine
from nxpm.errors import NxpmError
from nxpm.grouping import choose_preferred, group_packages, preferred_only
from nxpm.model import Package, parse_package_id, validate_query
from nxpm.transactions import TransactionManager


def package(**kwargs):
    data = {
        "id": "flatpak:user:app/org.example.App/x86_64/stable",
        "name": "Example",
        "source": "flatpak",
        "kind": "app",
        "appstream_id": "org.example.App",
    }
    data.update(kwargs)
    return Package(**data)


def test_package_ids_are_validated():
    assert parse_package_id("flatpak:user:app/org.example.App/x86_64/stable")[0] == "flatpak"
    assert parse_package_id("pacman:firefox")[1] == "firefox"
    for bad in ("pacman:bad name", "flatpak:user:../etc", "aur:pkg", ""):
        try:
            parse_package_id(bad)
        except NxpmError as exc:
            assert exc.code == "invalid-input"
        else:
            raise AssertionError(bad)


def test_query_rejects_control_characters():
    try:
        validate_query("firefox; rm")
    except NxpmError as exc:
        assert exc.code == "invalid-input"
    else:
        raise AssertionError("query")


def test_graphical_apps_prefer_flatpak():
    flatpak = package()
    native = package(
        id="pacman:example",
        source="pacman",
        kind="package",
        appstream_id="org.example.App",
    )
    preferred = choose_preferred([native, flatpak], ["flatpak", "pacman"])
    assert preferred.source == "flatpak"
    grouped = preferred_only(group_packages([native, flatpak], ["flatpak", "pacman"]))
    assert len(grouped) == 1
    assert grouped[0].source == "flatpak"
    assert "pacman" in grouped[0].summary


def test_libraries_keep_the_official_package():
    runtime = package(kind="runtime", id="flatpak:user:runtime/org.example.Runtime/x86_64/stable")
    native = package(id="pacman:example", source="pacman", kind="package", appstream_id="")
    preferred = choose_preferred([runtime, native], ["flatpak", "pacman"])
    assert preferred.source == "pacman"


def fake_gi(monkeypatch):
    """Stand in for PyGObject so the deferred path runs without a desktop."""
    import sys
    import types

    scheduled = []
    glib = types.SimpleNamespace(idle_add=lambda callback: scheduled.append(callback))
    repository = types.ModuleType("gi.repository")
    repository.GLib = glib
    gi = types.ModuleType("gi")
    gi.repository = repository
    monkeypatch.setitem(sys.modules, "gi", gi)
    monkeypatch.setitem(sys.modules, "gi.repository", repository)
    return scheduled


def test_pending_transaction_can_be_cancelled(monkeypatch):
    scheduled = fake_gi(monkeypatch)
    manager = TransactionManager()
    started = []

    def runner(transaction, _manager):
        started.append(transaction.id)

    first = manager.submit("install", "flatpak:user:app/org.example.App/x86_64/stable", "flatpak", runner)
    assert first.state == "succeeded"
    second = TransactionManager()
    second.defer = False
    queued = []

    def blocked(transaction, _manager):
        queued.append(transaction.state)

    # Cancel before the runner by removing it from a manager that has not pumped.
    manager = TransactionManager()
    manager.defer = True
    transaction = manager.submit("install", "flatpak:user:app/org.example.App/x86_64/stable", "flatpak", blocked)
    assert transaction.state == "pending"
    assert len(scheduled) == 1
    assert manager.cancel(transaction.id) is True
    assert transaction.state == "cancelled"
    assert manager.cancel(transaction.id) is False


def test_partial_pacman_updates_are_refused():
    engine = Engine(load_config(Path("/does/not/exist")), {"pacman": object(), "flatpak": object()})
    try:
        engine.update("pacman:bash")
    except NxpmError as exc:
        assert exc.code == "unsupported"
    else:
        raise AssertionError("partial update")


def test_aur_cannot_be_enabled_from_config(tmp_path: Path):
    path = tmp_path / "config.json"
    path.write_text(json.dumps({"sources": {"aur": {"enabled": True}, "flatpak": {"enabled": False}}}), encoding="utf-8")
    config = load_config(path)
    assert config.sources["aur"].enabled is False
    assert config.sources["aur"].implemented is False
    assert "flatpak" not in config.active_sources()
    assert "pacman" in config.active_sources()


class FakeFlatpak:
    source = "flatpak"

    def __init__(self):
        self.applied = []

    def search(self, query):
        return [package(name="Calculator")] if "calc" in query else []

    def list_installed(self):
        return []

    def list_updates(self):
        return []

    def details(self, package_id):
        return package(id=package_id)

    def apply(self, package_id, operation, progress):
        progress(0.5, "working")
        self.applied.append((operation, package_id))
        progress(1.0, "done")


def test_flatpak_install_uses_the_backend():
    backend = FakeFlatpak()
    engine = Engine(load_config(Path("/does/not/exist")), {"flatpak": backend})
    transaction = engine.install("flatpak:user:app/org.example.App/x86_64/stable", "flatpak")
    assert transaction.state == "succeeded"
    assert backend.applied == [("install", "flatpak:user:app/org.example.App/x86_64/stable")]
