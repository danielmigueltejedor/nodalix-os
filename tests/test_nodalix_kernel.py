from pathlib import Path
import runpy
import sys
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
BOOT = runpy.run_path(str(ROOT / 'updater/nodalix-kernel-boot'))
with patch.object(sys, 'path', [str(ROOT / 'tools'), *sys.path]):
    FINALIZE = runpy.run_path(str(ROOT / 'tools/finalize-release.py'))


class KernelBootTests(unittest.TestCase):
    def test_trial_preserves_microcode_and_encrypted_root_options(self):
        original = ('title Recovery\nlinux /vmlinuz-linux\ninitrd /amd-ucode.img\n'
                    'initrd /initramfs-linux.img\noptions rd.luks.name=abc=root root=/dev/mapper/root\n')
        result = BOOT['render_entry'](original)
        self.assertIn('linux /vmlinuz-linux-nodalix\n', result)
        self.assertIn('initrd /amd-ucode.img\ninitrd /initramfs-linux-nodalix.img\n', result)
        self.assertIn('options rd.luks.name=abc=root root=/dev/mapper/root\n', result)

    def test_trial_rejects_a_uki_or_an_already_managed_entry(self):
        with self.assertRaises(ValueError):
            BOOT['render_entry']('efi /EFI/Linux/arch.efi\n')
        with self.assertRaises(ValueError):
            BOOT['render_entry']('linux /vmlinuz-linux-nodalix\n')

    def test_promotion_requires_both_kernel_checks(self):
        flags = '{"SecurityAttributes":[{"AppstreamId":"org.fwupd.hsi.Kernel.Lockdown","Flags":["success"]}]}'
        with (patch('os.uname', return_value=SimpleNamespace(release='7.3.1-1-nodalix')),
              patch('subprocess.check_output', side_effect=['Current Entry: nodalix-linux.conf\n', flags]),
              patch('subprocess.run') as run):
            with self.assertRaisesRegex(ValueError, 'Both Linux'):
                BOOT['promote']()
        run.assert_not_called()


class KernelSigningTests(unittest.TestCase):
    def test_fixed_minisign_arguments_and_preservation_on_failed_verification(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            assets = root / 'assets'
            assets.mkdir()
            target = assets / 'nodalix-manifest.json'
            target.write_text('previous manifest')
            secret = root / '-s'
            public = root / '-p'
            secret.touch()
            public.touch()
            manifest = {'components': [{'package': 'linux-nodalix'}]}

            def minisign(command, *, cwd, check):
                self.assertTrue(check)
                self.assertFalse(cwd.is_relative_to(assets))
                if command[1] == '-Sm':
                    self.assertEqual(command[-2:], ['-s', 'release.key'])
                    self.assertEqual((cwd / 'release.key').resolve(), secret)
                    Path(command[4]).write_text('signature')
                else:
                    self.assertEqual(command[-2:], ['-p', 'release.pub'])
                    self.assertEqual((cwd / 'release.pub').resolve(), public)
                    raise RuntimeError('Verification failed')

            with patch('subprocess.run', side_effect=minisign) as command:
                with self.assertRaisesRegex(RuntimeError, 'Verification failed'):
                    FINALIZE['write_manifest'](assets, manifest, secret, public)
            self.assertEqual(command.call_count, 2)
            self.assertEqual(list(assets.iterdir()), [target])
            self.assertEqual(target.read_text(), 'previous manifest')

    def test_cannot_finalize_a_kernel_release_without_keys(self):
        with TemporaryDirectory() as directory:
            path = Path(directory)
            original = path / 'nodalix-manifest.json'
            original.write_text('candidate')
            with self.assertRaisesRegex(RuntimeError, 'requires a signing key'):
                FINALIZE['write_manifest'](path, {'components': [{'package': 'linux-nodalix'}]})
            self.assertEqual(original.read_text(), 'candidate')

    def test_private_key_must_never_be_in_the_assets_directory(self):
        with TemporaryDirectory() as directory:
            path = Path(directory)
            key = path / 'secret.key'
            public = path / 'release.pub'
            key.touch()
            public.touch()
            with self.assertRaisesRegex(RuntimeError, 'must be outside'):
                FINALIZE['write_manifest'](path, {'components': [{'package': 'linux-nodalix'}]}, key, public)
