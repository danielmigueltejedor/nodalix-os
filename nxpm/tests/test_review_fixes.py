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
