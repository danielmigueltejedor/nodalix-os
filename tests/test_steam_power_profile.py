"""Regression tests for Steam GameMode and temporary GNOME energy profiles."""

import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / "integrations/scripts/steam-auto-gamemode.py"
spec = importlib.util.spec_from_file_location("steam_auto_gamemode", SOURCE)
steam = importlib.util.module_from_spec(spec)
spec.loader.exec_module(steam)


class Variant:
    def __init__(self, _signature, args=None):
        if args is None:
            self.value = _signature
        else:
            self.value = args

    def unpack(self):
        return self.value


class FakeContext:
    def pending(self):
        return False

    def iteration(self, _block):
        raise AssertionError("No pending fake D-Bus events")


class FakeBus:
    def __init__(self):
        self.subscriptions = []
        self.holds = []
        self.releases = []
        self.fail_hold = False

    def signal_subscribe(self, *args):
        self.subscriptions.append(args)

    def call_sync(self, destination, path, interface, method, parameters,
                  return_type, flags, timeout, cancellable):
        assert destination == steam.PROFILE_BUS
        assert path == steam.PROFILE_PATH
        assert interface == steam.PROFILE_IFACE
        assert timeout > 0
        if method == "HoldProfile":
            if self.fail_hold:
                raise RuntimeError("profile unavailable")
            profile, reason, application = parameters.unpack()
            assert profile == "performance"
            assert reason
            assert application == "com.nodalix.SteamAutoGameMode"
            cookie = len(self.holds) + 1
            self.holds.append(cookie)
            return Variant((cookie,))
        if method == "ReleaseProfile":
            (cookie,) = parameters.unpack()
            self.releases.append(cookie)
            return Variant(())
        raise AssertionError("Unknown D-Bus method: " + method)


class SteamProfileTests(unittest.TestCase):
    def setUp(self):
        self.bus = FakeBus()
        self.gio = types.ModuleType("Gio")
        self.gio.BusType = types.SimpleNamespace(SYSTEM=1)
        self.gio.DBusSignalFlags = types.SimpleNamespace(NONE=0)
        self.gio.DBusCallFlags = types.SimpleNamespace(NONE=0)
        self.gio.bus_get_sync = lambda *_: self.bus
        self.glib = types.ModuleType("GLib")
        self.glib.Variant = Variant
        self.glib.VariantType = lambda signature: signature
        self.glib.MainContext = types.SimpleNamespace(default=FakeContext)
        self.gi = types.ModuleType("gi")
        self.repository = types.ModuleType("gi.repository")
        self.repository.Gio = self.gio
        self.repository.GLib = self.glib
        self.context = patch.dict(sys.modules, {
            "gi": self.gi, "gi.repository": self.repository,
        })
        self.context.start()
        self.addCleanup(self.context.stop)
        self.hold = steam.PerformanceProfileHold()

    def test_hold_only_while_games_running_and_release_on_last_exit(self):
        self.hold.sync(False)
        self.assertEqual([], self.bus.holds)
        self.hold.sync(True)
        self.assertEqual([1], self.bus.holds)
        self.assertEqual(2, len(self.bus.subscriptions))
        self.hold.sync(True)  # Second game, no duplicate reservation.
        self.assertEqual([1], self.bus.holds)
        self.hold.sync(False)
        self.assertEqual([1], self.bus.releases)
        self.hold.sync(True)  # Next gaming session.
        self.assertEqual([1, 2], self.bus.holds)

    def test_manual_user_override_is_not_overwritten_during_game(self):
        self.hold.sync(True)
        self.hold._profile_released(
            None, None, None, None, None, Variant((1,)), None
        )
        self.assertTrue(self.hold.user_overrode)
        self.hold.sync(True)
        self.assertEqual([1], self.bus.holds)
        self.hold.sync(False)
        self.hold.sync(True)
        self.assertEqual([1, 2], self.bus.holds)

    def test_daemon_restart_releases_obsolete_cookie_and_reacquires(self):
        self.hold.sync(True)
        self.hold._owner_changed(
            None, None, None, None, None,
            Variant((steam.PROFILE_BUS, ":1.20", ":1.21")), None,
        )
        self.hold.sync(True)
        self.assertEqual([1, 2], self.bus.holds)
        self.hold.sync(False)
        self.assertEqual([2], self.bus.releases)

    def test_first_daemon_activation_keeps_the_hold(self):
        self.hold.sync(True)
        self.hold._owner_changed(
            None, None, None, None, None,
            Variant((steam.PROFILE_BUS, "", ":1.20")), None,
        )
        self.hold.sync(True)
        self.hold.sync(False)
        self.assertEqual([1], self.bus.holds)
        self.assertEqual([1], self.bus.releases)

    def test_missing_profile_does_not_crash_gamemode(self):
        self.bus.fail_hold = True
        self.hold.sync(True)
        self.assertIsNone(self.hold.cookie)
        self.assertGreater(self.hold.retry_after, 0)
        self.hold.sync(False)  # Reset the backoff at end of gaming session.
        self.bus.fail_hold = False
        self.hold.sync(True)
        self.assertEqual([1], self.bus.holds)

    def test_unknown_cookie_does_not_interrupt_hold(self):
        self.hold.sync(True)
        self.hold._profile_released(
            None, None, None, None, None, Variant((9999,)), None
        )
        self.assertEqual(1, self.hold.cookie)
        self.assertFalse(self.hold.user_overrode)

    def test_integration_package_provides_energy_backend(self):
        root = Path(__file__).resolve().parents[1]
        pkg = (root / "packaging/nodalix-integrations/PKGBUILD").read_text()
        script = SOURCE.read_text()
        self.assertIn("power-profiles-daemon: GNOME performance profile holds", pkg)
        self.assertIn("power.sync(bool(tracked))", script)
        self.assertIn("power.release()", script)


if __name__ == "__main__":
    unittest.main()
