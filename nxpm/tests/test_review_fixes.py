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


def test_engine_routes_pacman_removal_through_the_installed_helper(monkeypatch, tmp_path):
    import subprocess

    from nxpm.engine import Engine

    helper = tmp_path / "remove-package"
    helper.write_text("#!/bin/sh\n")
    calls = []

    def fake_run(command, **_kwargs):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.delenv("NXPM_NO_PKEXEC", raising=False)
    monkeypatch.setattr("nxpm.engine.HELPER_DIR", str(tmp_path))
    monkeypatch.setattr(subprocess, "run", fake_run)
    monkeypatch.setattr("os.geteuid", lambda: 1000)
    engine = Engine(load_config(Path("/does/not/exist")), {"pacman": object()})
    transaction = engine.remove("pacman:bash", "pacman")
    assert transaction.state == "succeeded"
    assert calls == [["pkexec", str(helper), "pacman:bash"]]
    with pytest.raises(NxpmError):
        engine.install("pacman:bash", "pacman")


def test_privileged_work_needs_the_installed_helper(monkeypatch, tmp_path):
    from nxpm.engine import Engine

    monkeypatch.delenv("NXPM_NO_PKEXEC", raising=False)
    monkeypatch.setattr("nxpm.engine.HELPER_DIR", str(tmp_path))
    monkeypatch.setattr("os.geteuid", lambda: 1000)
    engine = Engine(load_config(Path("/does/not/exist")), {"pacman": object()})
    transaction = engine.remove("pacman:bash", "pacman")
    assert transaction.state == "failed"
    assert "installed system-wide" in transaction.error


def test_the_helper_directory_cannot_be_chosen_from_the_environment(monkeypatch, tmp_path):
    import subprocess

    from nxpm.engine import Engine

    (tmp_path / "remove-package").write_text("#!/bin/sh\n")
    calls = []
    monkeypatch.delenv("NXPM_NO_PKEXEC", raising=False)
    # A hostile NXPM_HELPER_DIR must not redirect pkexec, even when the real
    # packaged helper directory is absent from this test environment.
    monkeypatch.setattr("nxpm.engine.HELPER_DIR", str(tmp_path / "missing-system-helpers"))
    monkeypatch.setenv("NXPM_HELPER_DIR", str(tmp_path))
    monkeypatch.setattr(subprocess, "run", lambda command, **_k: calls.append(command))
    monkeypatch.setattr("os.geteuid", lambda: 1000)
    engine = Engine(load_config(Path("/does/not/exist")), {"pacman": object()})
    assert engine.remove("pacman:bash", "pacman").state == "failed"
    assert calls == []


def test_install_session_leaves_packaged_services_alone(monkeypatch, tmp_path, capsys):
    from nxpm import session

    packaged = tmp_path / "nodalix-nxpm"
    (packaged / "lib").mkdir(parents=True)
    monkeypatch.setattr(session, "PACKAGED_ROOT", packaged)
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "data"))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "config"))
    session.install_session()
    assert not (tmp_path / "config").exists()
    assert "already in place" in capsys.readouterr().out


def test_install_session_removes_only_generated_user_overrides(monkeypatch, tmp_path, capsys):
    from nxpm import session

    packaged = tmp_path / "nodalix-nxpm"
    (packaged / "lib").mkdir(parents=True)
    monkeypatch.setattr(session, "PACKAGED_ROOT", packaged)
    data, config = tmp_path / "data", tmp_path / "config"
    monkeypatch.setenv("XDG_DATA_HOME", str(data))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(config))
    (data / "dbus-1" / "services").mkdir(parents=True)
    (config / "systemd" / "user").mkdir(parents=True)
    stale = data / "dbus-1" / "services" / "os.nodalix.Nxpm.service"
    stale.write_text("[D-BUS Service]\nExec=/venv/bin/python -m nxpm daemon\n")
    custom = config / "systemd" / "user" / "nxpm-check.timer"
    custom.write_text("[Timer]\nOnBootSec=1h\n")
    session.install_session()
    out = capsys.readouterr().out
    assert not stale.exists()
    assert custom.exists()
    assert "Removed outdated user override" in out
    assert "already in place" in out


def test_update_notification_uses_the_translated_store_name():
    from nxpm.check import store_text

    assert store_text({"LANG": "es_ES.UTF-8"})[0] == "Tienda"
    assert store_text({"LANGUAGE": "ca:es", "LANG": "en_US.UTF-8"})[0] == "Botiga"
    assert store_text({"LANGUAGE": "en:es", "LANG": "es_ES.UTF-8"})[0] == "Store"
    assert store_text({"LC_ALL": "C", "LANG": "es_ES.UTF-8"})[0] == "Store"  # C locale wins
    assert store_text({"LANG": "en_US.UTF-8"})[0] == "Store"
    assert store_text({"LANG": "xx_YY"})[0] == "Store"
    assert store_text({})[0] == "Store"
    assert "{count}" in store_text({"LANG": "es_ES"})[2]


class FakeClock:
    def __init__(self):
        self.now = 0.0

    def __call__(self):
        return self.now


def test_idle_service_exits_only_when_it_has_nothing_to_remember():
    from nxpm.service import IdleWatch
    from nxpm.transactions import TransactionManager

    clock = FakeClock()
    manager = TransactionManager()
    watch = IdleWatch(manager, timeout=100, clock=clock)
    clock.now = 99
    assert not watch.expired()
    watch.touch()
    clock.now = 198
    assert not watch.expired()
    clock.now = 199
    assert watch.expired()

    manager.submit("install", "a", "flatpak", lambda tx, mgr: None)
    clock.now = 10_000
    assert not watch.expired()  # history is kept in memory only


def test_busy_service_never_exits():
    from nxpm.service import IdleWatch
    from nxpm.transactions import TransactionManager

    clock = FakeClock()
    manager = TransactionManager()
    watch = IdleWatch(manager, timeout=1, clock=clock)
    clock.now = 50
    manager._active = "running"
    assert manager.busy() and not watch.expired()
    manager._active = None
    assert watch.expired()
