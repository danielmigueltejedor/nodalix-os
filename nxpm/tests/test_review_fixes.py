import json
import stat
from pathlib import Path

import pytest

from nxpm.config import load_config
from nxpm.errors import NxpmError
from nxpm.foreign import _neutralize
from nxpm.model import parse_package_id, validate_query
from nxpm.permissions import classify
from nxpm.transactions import TransactionManager
from nxpm.updates_appimage import _release_endpoint


@pytest.mark.parametrize("bad", ["pacman:firefox\n", "appimage:Demo\n", "flatpak:user:app/a.b/x86_64/stable\n"])
def test_identifiers_with_a_trailing_newline_are_rejected(bad):
    with pytest.raises(NxpmError):
        parse_package_id(bad)


def test_query_with_an_embedded_newline_is_rejected():
    with pytest.raises(NxpmError):
        validate_query("fire\nfox")


def test_negated_permissions_are_not_grants():
    assert classify("filesystems", "!home") == "low"
    assert classify("filesystems", "home") == "high"


def test_wildcard_bus_names_are_high_risk():
    assert classify("bus", "org.freedesktop.*=talk") == "high"
    assert classify("bus", "org.freedesktop.Notifications=talk") == "medium"


def test_extraction_keeps_execute_bits_and_drops_special_bits(tmp_path: Path):
    tool = tmp_path / "helper"
    tool.write_bytes(b"x")
    tool.chmod(0o4777)
    data = tmp_path / "data.txt"
    data.write_bytes(b"x")
    data.chmod(0o666)
    _neutralize(tmp_path)
    assert stat.S_IMODE(tool.stat().st_mode) == 0o755
    assert stat.S_IMODE(data.stat().st_mode) == 0o644


def test_config_with_a_non_object_top_level_value_uses_defaults(tmp_path: Path):
    path = tmp_path / "config.json"
    path.write_text(json.dumps(["not", "an", "object"]), encoding="utf-8")
    assert "pacman" in load_config(path).active_sources()


def test_cancelling_a_finished_transaction_does_not_duplicate_history():
    manager = TransactionManager()
    transaction = manager.submit("install", "flatpak:user:app/org.example.App/x86_64/stable", "flatpak", lambda t, m: None)
    assert transaction.state == "succeeded"
    before = len(manager.history(200))
    assert manager.cancel(transaction.id) is False
    assert len(manager.history(200)) == before


def test_a_failing_listener_does_not_stall_the_queue():
    manager = TransactionManager()

    def broken(_transaction):
        raise RuntimeError("signal failed")

    manager.add_listener(broken)
    first = manager.submit("install", "flatpak:user:app/org.example.A/x86_64/stable", "flatpak", lambda t, m: None)
    second = manager.submit("install", "flatpak:user:app/org.example.B/x86_64/stable", "flatpak", lambda t, m: None)
    assert first.state == "succeeded"
    assert second.state == "succeeded"


def test_release_channels():
    assert _release_endpoint("latest") == "releases/latest"
    assert _release_endpoint("v1.2.3") == "releases/tags/v1.2.3"
    for unsupported in ("latest-pre", "latest-all"):
        with pytest.raises(NxpmError):
            _release_endpoint(unsupported)


class _Local:
    def __init__(self, names):
        self.names = names

    def get_pkg(self, name):
        return object() if name in self.names else None

    @property
    def pkgcache(self):
        return []


class _RemovalTransaction:
    def __init__(self, fail_prepare=False):
        self.fail_prepare = fail_prepare
        self.steps = []

    def remove_pkg(self, pkg):
        self.steps.append("remove")

    def prepare(self):
        if self.fail_prepare:
            raise RuntimeError("could not satisfy dependencies")
        self.steps.append("prepare")

    def commit(self):
        self.steps.append("commit")

    def release(self):
        self.steps.append("release")


class _RemovalHandle:
    def __init__(self, transaction, installed=("bash",)):
        self.transaction = transaction
        self.local = _Local(installed)

    def get_localdb(self):
        return self.local

    def init_transaction(self):
        return self.transaction


def _as_root(monkeypatch, tmp_path):
    import os

    monkeypatch.setattr(os, "geteuid", lambda: 0)
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path))


def test_removing_an_official_package_uses_libalpm(monkeypatch, tmp_path):
    from nxpm.backends.pacman import PacmanBackend

    _as_root(monkeypatch, tmp_path)
    transaction = _RemovalTransaction()
    backend = PacmanBackend(handle_factory=lambda: _RemovalHandle(transaction))
    backend.remove_package("pacman:bash", lambda *_: None)
    assert transaction.steps == ["remove", "prepare", "commit", "release"]


def test_removal_is_refused_when_other_packages_need_it(monkeypatch, tmp_path):
    from nxpm.backends.pacman import PacmanBackend

    _as_root(monkeypatch, tmp_path)
    transaction = _RemovalTransaction(fail_prepare=True)
    backend = PacmanBackend(handle_factory=lambda: _RemovalHandle(transaction))
    with pytest.raises(NxpmError) as error:
        backend.remove_package("pacman:bash", lambda *_: None)
    assert error.value.code == "failed"
    assert "commit" not in transaction.steps
    assert transaction.steps[-1] == "release"


def test_removal_needs_root_and_an_installed_package(monkeypatch, tmp_path):
    import os

    from nxpm.backends.pacman import PacmanBackend

    backend = PacmanBackend(handle_factory=lambda: _RemovalHandle(_RemovalTransaction()))
    monkeypatch.setattr(os, "geteuid", lambda: 1000)
    with pytest.raises(NxpmError) as error:
        backend.remove_package("pacman:bash", lambda *_: None)
    assert error.value.code == "authorization-required"
    _as_root(monkeypatch, tmp_path)
    with pytest.raises(NxpmError) as error:
        backend.remove_package("pacman:missing", lambda *_: None)
    assert error.value.code == "not-found"


def test_engine_routes_pacman_removal_through_authorization(monkeypatch):
    import subprocess
    import sys

    from nxpm.engine import Engine

    calls = []

    def fake_run(command, **_kwargs):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.delenv("NXPM_NO_PKEXEC", raising=False)
    monkeypatch.setattr(subprocess, "run", fake_run)
    monkeypatch.setattr("os.geteuid", lambda: 1000)
    engine = Engine(load_config(Path("/does/not/exist")), {"pacman": object()})
    transaction = engine.remove("pacman:bash", "pacman")
    assert transaction.state == "succeeded"
    assert calls == [["pkexec", sys.executable, "-m", "nxpm", "privileged-remove", "pacman:bash"]]
    with pytest.raises(NxpmError):
        engine.install("pacman:bash", "pacman")
