"""Bounded QMP suspend protocol and guest-only desktop-cycle safety checks."""
import importlib.util
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('ci_wake_guest', ROOT / 'tools/ci-wake-guest.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NativeGuestCycleTests(unittest.TestCase):
    def exchange(self, events, expected_command=None):
        client, server = socket.socketpair()
        client.settimeout(3)
        server.settimeout(3)
        failures = []
        received = []

        def qemu():
            try:
                with server, server.makefile('rwb') as stream:
                    stream.write(b'{"QMP":{}}\n')
                    stream.flush()
                    received.append(stream.readline())
                    stream.write(b'{"return":{}}\n' + events)
                    stream.flush()
                    if expected_command:
                        received.append(stream.readline())
            except Exception as error:
                failures.append(error)

        worker = threading.Thread(target=qemu)
        worker.start()
        try:
            with client:
                MODULE.wake_guest(client, delay=0)
        finally:
            worker.join(timeout=4)
        self.assertFalse(worker.is_alive())
        if failures:
            raise failures[0]
        expected = [b'{"execute":"qmp_capabilities"}\n']
        if expected_command:
            expected.append(expected_command)
        self.assertEqual(received, expected)

    def test_actual_suspend_and_wakeup_events_are_both_required(self):
        self.exchange(b'{"event":"SUSPEND"}\n{"event":"WAKEUP"}\n',
                      b'{"execute":"system_wakeup"}\n')

    def test_shutdown_without_suspend_is_a_failure(self):
        with self.assertRaisesRegex(RuntimeError, 'did not complete'):
            self.exchange(b'{"event":"SHUTDOWN"}\n')

    def test_suspend_without_wakeup_is_a_failure(self):
        with self.assertRaisesRegex(RuntimeError, 'did not complete'):
            self.exchange(b'{"event":"SUSPEND"}\n', b'{"execute":"system_wakeup"}\n')

    def test_qmp_errors_are_failures(self):
        with self.assertRaisesRegex(RuntimeError, 'GenericError'):
            self.exchange(b'{"error":{"class":"GenericError"}}\n')

    def test_cycle_cannot_run_on_host_by_default(self):
        environment = dict(os.environ)
        environment.pop('NODALIX_CI_GUEST', None)
        result = subprocess.run([sys.executable, str(ROOT / 'tools/ci-native-shell-cycle.py')],
                                env=environment, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Disposable CI guest required', result.stderr)
