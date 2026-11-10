"""The root helper must never decode attacker-influenced pixels as root."""
import importlib.machinery
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader('sync_login_background', str(ROOT/'gnome/settings/sync-login-background'))
spec = importlib.util.spec_from_loader(loader.name, loader)
helper = importlib.util.module_from_spec(spec)
loader.exec_module(helper)
JPEG = b'\xff\xd8\xff\xe0' + b'0' * 32


class DecoderPrivilegeTests(unittest.TestCase):
    def run_decode(self, euid, stdout=JPEG):
        completed = subprocess.CompletedProcess([], 0, stdout=stdout, stderr=b'')
        with patch.object(helper.os, 'geteuid', return_value=euid), \
             patch.object(helper.pwd, 'getpwnam', return_value=helper.pwd.struct_passwd(
                 ('nobody', 'x', 65534, 65534, '', '/', '/usr/bin/nologin'))), \
             patch.object(helper.subprocess, 'run', return_value=completed) as run:
            result = helper.decode(JPEG)
        return result, run

    def test_root_decodes_as_nobody_with_limits_and_a_clean_environment(self):
        result, run = self.run_decode(0)
        self.assertEqual(result, JPEG)
        options = run.call_args.kwargs
        self.assertEqual((options['user'], options['group'], options['extra_groups']), (65534, 65534, []))
        self.assertIs(options['preexec_fn'], helper.limit_decoder)
        self.assertEqual(options['env'], {'PATH': '/usr/bin', 'LC_ALL': 'C'})
        self.assertEqual(options['cwd'], '/')
        self.assertEqual(options['timeout'], 20)
        command = run.call_args.args[0]
        self.assertEqual(command[command.index('-protocol_whitelist') + 1], 'pipe')  # no file or network access
        self.assertEqual(command[-3:], ['-f', 'mjpeg', 'pipe:1'])  # nothing is written by the decoder

    def test_unprivileged_runs_do_not_try_to_switch_user(self):
        _, run = self.run_decode(1000)
        self.assertNotIn('user', run.call_args.kwargs)

    def test_decoder_output_is_validated_by_the_privileged_process(self):
        with self.assertRaises(ValueError):
            self.run_decode(0, stdout=b'not a jpeg')
        with self.assertRaises(ValueError):
            self.run_decode(0, stdout=b'')

    def test_limits_cover_memory_time_and_files(self):
        applied = {}
        with patch.object(helper.resource, 'setrlimit', side_effect=lambda name, value: applied.update({name: value})):
            helper.limit_decoder()
        for name in ('RLIMIT_AS', 'RLIMIT_CPU', 'RLIMIT_FSIZE', 'RLIMIT_NOFILE', 'RLIMIT_CORE'):
            self.assertIn(getattr(helper.resource, name), applied)
        self.assertEqual(applied[helper.resource.RLIMIT_CORE], (0, 0))

    def test_publish_rejects_non_jpeg_input_before_running_anything(self):
        with tempfile.TemporaryDirectory() as folder, patch.object(helper.subprocess, 'run') as run:
            for data in (b'\x89PNG' + b'0' * 8, b'', JPEG + b'0' * (20 * 1024 * 1024)):
                with self.assertRaises(ValueError):
                    helper.publish(data, Path(folder))
        run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
