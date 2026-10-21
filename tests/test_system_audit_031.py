"""Regressions found while auditing the installed Nodalix desktop."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from test_gnome_transition_030 import SYSTEM
from test_phone_resilience import load, BusError

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('native_builder', ROOT/'tools/build-settings-packages.py')
BUILDER = importlib.util.module_from_spec(spec)
spec.loader.exec_module(BUILDER)


class BuildSourceTests(unittest.TestCase):
    def test_transient_clone_failure_retries_without_publishing_partial_source(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            upstream = root/'upstream'
            subprocess.run(['git', 'init', '-q', str(upstream)], check=True)
            (upstream/'file').write_text('pinned source')
            subprocess.run(['git', '-C', str(upstream), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(upstream), '-c', 'user.name=Test', '-c',
                            'user.email=test@example.invalid', 'commit', '-qm', 'source'], check=True)
            revision = subprocess.check_output(['git', '-C', str(upstream), 'rev-parse', 'HEAD'], text=True).strip()
            real_run = BUILDER.run
            attempts = []
            def run(args, **kwargs):
                if args[:2] == ['git', 'clone']:
                    attempts.append(args)
                    if len(attempts) == 1:
                        Path(args[-1]).mkdir()
                        (Path(args[-1])/'partial').write_text('partial transfer')
                        raise subprocess.CalledProcessError(128, args)
                return real_run(args, **kwargs)
            with mock.patch.object(BUILDER, 'run', side_effect=run), mock.patch.object(BUILDER.time, 'sleep'):
                target = BUILDER.checkout(None, root/'cache', 'source', str(upstream), revision)
            self.assertEqual(len(attempts), 2)
            self.assertEqual((target/'file').read_text(), 'pinned source')
            self.assertFalse((target/'partial').exists())
            self.assertEqual(list((root/'cache').iterdir()), [target])


class LegacyUnitTests(unittest.TestCase):
    def test_completed_030_migration_still_retires_quickshell_update_override(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            old = root/'etc/systemd/user/nodalix-update-apps.service'
            new = root/'usr/lib/systemd/user/nodalix-update-apps.service'
            for p in (old, new):p.parent.mkdir(parents=True, exist_ok=True)
            text = '[Service]\nExecStart=%h/.config/quickshell/scripts/update/nodalix-update-apps.sh\n'
            old.write_text(text)
            new.write_text('[Service]\nExecStart=/usr/bin/nodalix-system-update user-apps\n')
            marker = root/'var/lib/nodalix-updater/migrations/gnome-0.3.0/completed.json'
            marker.parent.mkdir(parents=True)
            marker.write_text('{}')
            SYSTEM.migrate(root, runner=mock.Mock())
            SYSTEM.migrate(root, runner=mock.Mock())
            self.assertFalse(old.exists())
            saved = root/'var/lib/nodalix-updater/migrations/gnome-0.3.1/files/etc/systemd/user/nodalix-update-apps.service'
            self.assertEqual(saved.read_text(), text)


class AutomaticUpdateTests(unittest.TestCase):
    def test_missing_dns_defers_without_running_pacman(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for name, text in {'id': 'echo 0', 'getent': 'exit 2',
                               'pacman': 'echo unexpected > "'+str(root/'pacman-ran')+'"'}.items():
                helper = root/name
                helper.write_text('#!/bin/sh\n'+text+'\n')
                helper.chmod(0o755)
            result = subprocess.run(['/bin/sh', str(ROOT/'updater/nodalix-system-update'),
                                     'privileged-system'], capture_output=True, text=True,
                                    env={**os.environ, 'PATH': str(root)})
            self.assertEqual(result.returncode, 75)
            self.assertFalse((root/'pacman-ran').exists())

    def test_package_failures_are_not_treated_as_network_deferrals(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for name, text in {'id': 'echo 0', 'getent': 'exit 0', 'pacman': 'exit 1'}.items():
                helper = root/name;helper.write_text('#!/bin/sh\n'+text+'\n');helper.chmod(0o755)
            result = subprocess.run(['/bin/sh', str(ROOT/'updater/nodalix-system-update'),
                                     'privileged-system'], capture_output=True,
                                    env={**os.environ, 'PATH': str(root)})
            self.assertEqual(result.returncode, 1)


class PhoneReconnectTests(unittest.TestCase):
    def manager(self):
        props, proxy = mock.Mock(), mock.Mock()
        props.Get.return_value = False
        module = load('daemon.py', ['Daemon'], config=SimpleNamespace(
            ADAPTER='hci0', IPHONE_MAC='00:11:22:33:44:55', NOTIFICATIONS_ENABLED=False),
            bluez=lambda path, iface: props if iface.endswith('Properties') else proxy,
            time=SimpleNamespace(monotonic=lambda: 100), RECONNECT_TICK_SEC=15,
            RECONNECT_MAX_SEC=120)
        manager = module.Daemon.__new__(module.Daemon)
        manager._reconnect_in_flight = False
        manager._reconnect_next = 0
        manager._reconnect_backoff = 15
        manager._recover_hfp_now = mock.Mock()
        return manager, proxy

    def test_pending_reconnect_is_async_and_not_duplicated(self):
        manager, proxy = self.manager()
        self.assertTrue(manager._maintain_phone_connection())
        self.assertTrue(manager._reconnect_in_flight)
        manager._maintain_phone_connection()
        proxy.Connect.assert_called_once()
        self.assertEqual(proxy.Connect.call_args.kwargs['timeout'], 12)
        proxy.Connect.call_args.kwargs['reply_handler']()
        self.assertFalse(manager._reconnect_in_flight)
        manager._recover_hfp_now.assert_called_once()

    def test_reconnect_failure_releases_pending_state_and_backs_off(self):
        manager, proxy = self.manager()
        manager._maintain_phone_connection()
        proxy.Connect.call_args.kwargs['error_handler'](BusError())
        self.assertFalse(manager._reconnect_in_flight)
        self.assertEqual(manager._reconnect_next, 115)
        self.assertEqual(manager._reconnect_backoff, 30)


class TilingRestoreTests(unittest.TestCase):
    def test_real_gio_restores_saved_window_state_and_ignores_corrupt_json(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            folder = root/'tiling-assistant'
            folder.mkdir()
            saved = folder/'tiledSessionRestore2.json'
            saved.write_text(json.dumps({'windows': {'1': {}}, 'tileGroups': {'1': [1]}}))
            source = ROOT/'gnome/extensions/tiling-assistant@leleat-on-github/extension.js'
            script = root/'restore.mjs'
            script.write_text('''import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
Gio._promisify(Gio.File.prototype, 'load_contents_async');
const source = new TextDecoder().decode(Gio.File.new_for_path(ARGV[0]).load_contents(null)[1]);
const marker = 'async _loadAfterSessionLock(cancellable) {';
const body = source.slice(source.indexOf(marker) + marker.length, source.lastIndexOf('\\n    }'));
const global = {display: {list_all_windows: () => []}};
const logError = () => {};
const restore = eval('(async function(cancellable) {' + body + '})');
let groups, windows;
const context = {_wasLocked: true, _twm: {setTileGroups: v => groups=v, setTileStates: v => windows=v}};
await restore.call(context, null);
if (groups?.size !== 1 || windows?.size !== 1) throw new Error('Saved session was not restored');
Gio.File.new_for_path(ARGV[1]).replace_contents('{broken', null, false, Gio.FileCreateFlags.REPLACE_DESTINATION, null);
context._wasLocked = true;
await restore.call(context, null);
''')
            subprocess.run(['gjs', '-m', str(script), str(source), str(saved)], check=True,
                           env={**os.environ, 'XDG_CONFIG_HOME': str(root)}, capture_output=True)


if __name__ == '__main__':
    unittest.main()
