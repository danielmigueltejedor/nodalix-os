"""Exercise production lifecycle code with unavailable external D-Bus services."""
import ast
import logging
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[1] / 'phone-link/src/iphonebridge'

class BusError(Exception):
    def __init__(self, name='org.freedesktop.DBus.Error.ServiceUnknown'):
        self.name = name
    def get_dbus_name(self): return self.name
    def get_dbus_message(self): return 'Maximum advertisements reached'

def load(filename, names, **scope):
    tree = ast.parse((ROOT / filename).read_text())
    nodes = [ast.ImportFrom(module='__future__', names=[ast.alias(name='annotations')], level=0)]
    nodes += [n for n in tree.body if isinstance(n, (ast.ClassDef, ast.FunctionDef)) and n.name in names]
    env = dict(log=logging.getLogger('resilience'), dbus=SimpleNamespace(exceptions=SimpleNamespace(DBusException=BusError)), **scope)
    exec(compile(ast.fix_missing_locations(ast.Module(body=nodes, type_ignores=[])), filename, 'exec'), env)
    return SimpleNamespace(**env)

class PhoneResilience(unittest.TestCase):
    def hfp(self):
        bus = Mock()
        dbus = SimpleNamespace(exceptions=SimpleNamespace(DBusException=BusError), Interface=lambda obj, _: obj)
        # Interface is injected after loading to keep the helper defaults simple.
        mod = load('hfp/ofono_client.py', ['HfpManager', '_safe_remove'], system_bus=bus, OFONO='org.ofono', _MGR_IFACE='org.ofono.Manager')
        mod.HfpManager.start.__globals__['dbus'] = dbus
        return mod.HfpManager(Mock()), bus

    def test_missing_ofono_proxy_and_getmodems_are_nonfatal(self):
        for stage in ('proxy', 'query'):
            with self.subTest(stage=stage):
                manager, bus = self.hfp()
                if stage == 'proxy': bus.get_object.side_effect = BusError()
                else: bus.get_object.return_value.GetModems.side_effect = BusError()
                self.assertFalse(manager.start())
                self.assertEqual(manager._mgr_matches, [])
                self.assertIsNone(manager._modem_path)

    def test_partial_subscription_is_cleaned_and_retry_recovers(self):
        manager, bus = self.hfp()
        proxy = bus.get_object.return_value
        proxy.GetModems.return_value = []
        subscription = Mock()
        proxy.connect_to_signal.side_effect = [subscription, BusError()]
        self.assertFalse(manager.start())
        subscription.remove.assert_called_once()
        proxy.connect_to_signal.side_effect = None
        self.assertTrue(manager.start())
        count = proxy.connect_to_signal.call_count
        self.assertTrue(manager.start())
        self.assertEqual(proxy.connect_to_signal.call_count, count)
        manager.stop()

    def test_advertising_capacity_timeout_and_proxy_failure(self):
        for name in ('org.bluez.Error.NotPermitted', 'org.bluez.Error.Failed', 'org.freedesktop.DBus.Error.NoReply', 'org.freedesktop.DBus.Error.ServiceUnknown'):
            for stage in ('proxy', 'register'):
                with self.subTest(name=name, stage=stage):
                    bluez = Mock()
                    if stage == 'proxy': bluez.side_effect = BusError(name)
                    else: bluez.return_value.RegisterAdvertisement.side_effect = BusError(name)
                    mod = load('bluez_setup.py', ['register_advert'], _advert_instance=object(), _AncsAdvert=SimpleNamespace(PATH='/advert'), bluez=bluez, config=SimpleNamespace(ADAPTER='hci0'))
                    mod.register_advert.__globals__['dbus'].Dictionary = lambda *a, **k: {}
                    self.assertFalse(mod.register_advert())

    def test_start_keeps_ancs_map_pbap_and_service_alive(self):
        glib = Mock(); glib.timeout_add_seconds.side_effect = range(1, 20)
        ancs, hfp, sessions, listener = Mock(), Mock(), Mock(), Mock()
        hfp.start.side_effect = BusError()
        contacts = Mock(); contacts.count.return_value = 1
        bluez = Mock()
        bluez.return_value.Get.return_value = 'le'
        system_bus = Mock()
        mod = load('daemon.py', ['Daemon'], SessionManager=Mock(return_value=sessions), ContactsResolver=Mock(return_value=contacts),
                   config=SimpleNamespace(ensure_dirs=Mock(), ADAPTER='hci0', IPHONE_MAC='00:11', NOTIFICATIONS_ENABLED=True, CALLS_ENABLED=True, AUTO_RECONNECT=False, COPY_VERIFICATION_CODES=False),
                   bluez=bluez, system_bus=system_bus,
                   bluez_setup=SimpleNamespace(prepare=Mock(side_effect=BusError())), GLib=glib, SESSION_RETRY_SEC=60, RECONNECT_TICK_SEC=15, CONTACTS_REFRESH_SEC=86400,
                   AncsClient=Mock(return_value=ancs), HfpManager=Mock(return_value=hfp), JsonlSink=Mock(), LibnotifySink=Mock(),
                   MapEventListener=Mock(return_value=listener), claim_bus_name=Mock(), MessagesService=Mock(), signal=SimpleNamespace(SIGINT=2, SIGTERM=15, signal=Mock()), SessionError=RuntimeError)
        daemon = mod.Daemon()
        daemon.start()
        ancs.start.assert_called_once()
        sessions.open_all.assert_called_once()
        listener.start.assert_called_once()
        self.assertTrue(daemon._post_sessions_done)
        self.assertIsNotNone(daemon._dbus_service)
        self.assertIsNotNone(daemon._hfp_retry_id)
        self.assertIsNotNone(daemon._bluetooth_retry_id)
        hfp.start.side_effect = None; hfp.start.return_value = True
        self.assertFalse(daemon._retry_hfp())
        self.assertIsNone(daemon._hfp_retry_id)
        mod.bluez_setup.prepare.side_effect = None; mod.bluez_setup.prepare.return_value = True
        self.assertFalse(daemon._retry_bluetooth())
        self.assertIsNone(daemon._bluetooth_retry_id)

if __name__ == '__main__': unittest.main()
