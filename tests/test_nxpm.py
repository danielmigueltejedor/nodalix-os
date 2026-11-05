"""NXPM never compiles on a client, never trusts forged removal requests."""
import runpy
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]
API = runpy.run_path(str(ROOT / 'nxpm/nxpm'), run_name='test_nxpm')
G = API['install'].__globals__


class NxpmTests(unittest.TestCase):
    def test_any_native_package_is_supported_without_catalogue(self):
        txn = Mock(return_value=0)
        with patch.dict(G, {'native_transaction': txn}):
            self.assertEqual(API['install']('inkscape', 'native'), 0)
        txn.assert_called_once_with(['--needed', '--', 'inkscape'])

    def test_flatpak_install_never_builds_source(self):
        command = Mock(return_value=0)
        with patch.dict(G, {'execute': command}):
            self.assertEqual(API['install']('flatpak:org.gnome.Boxes', 'native'), 0)
        command.assert_called_once_with(
            ['/usr/bin/flatpak', 'install', '--user', '--', 'org.gnome.Boxes'])

    def test_invalid_package_is_rejected_before_process(self):
        command = Mock()
        with patch.dict(G, {'execute': command, 'native_transaction': command}):
            for name in ['--overwrite', '../tmp', 'foo;touch', 'aur:unsafe']:
                with self.assertRaises(API['NxpmError']):
                    API['install'](name, 'native')
        command.assert_not_called()

    def test_critical_packages_cannot_be_removed(self):
        cmd = Mock()
        with patch.dict(G, {'execute': cmd}):
            with self.assertRaises(API['NxpmError']):
                API['remove']('nodalix-release', 'native')
        cmd.assert_not_called()

    def test_flatpak_permissions_deny_unlisted_scopes(self):
        cmd = Mock()
        with patch.dict(G, {'execute': cmd, 'flatpak_installed': lambda _: '--user'}):
            with self.assertRaises(API['NxpmError']):
                API['permissions']('grant', 'org.gnome.Boxes', 'host')
            API['permissions']('grant', 'org.gnome.Boxes', 'xdg-download')
        # Mock's return value is what permissions returns.
        # Verify the requested permission is a known, bounded scope.
        self.assertEqual(cmd.call_args.args[0][-2], '--filesystem=xdg-download')

    def test_native_update_keeps_release_kernel_selected(self):
        command = Mock(return_value=0)
        with patch.dict(G, {'execute': command}):
            self.assertEqual(API['native_transaction'](['--needed', '--', 'inkscape']), 0)
        self.assertIn('linux-nodalix', command.call_args.args[0])
        self.assertNotIn('makepkg', str(command.call_args))

    def test_remove_desktop_checks_ownership_and_package(self):
        from types import SimpleNamespace
        remover = Mock(return_value=0)
        with patch.object(Path, "is_file", return_value=True), \
             patch.object(Path, "is_symlink", return_value=False), \
             patch.object(Path, "stat", return_value=SimpleNamespace(st_uid=0, st_mode=0o755)), \
             patch.dict(G, {'output': lambda command: (0, 'inkscape\n'),
                            'remove': remover}):
            self.assertEqual(API['remove_desktop']('inkscape.desktop'), 0)
        remover.assert_called_once_with('inkscape', 'native')

    def test_old_apps_are_absent_from_new_package(self):
        pkg = (ROOT / 'packaging/nxpm/PKGBUILD').read_text()
        self.assertNotIn('nodalix-apps/bin', pkg)
        self.assertNotIn('apps/*.toml', pkg)
        for text in ('cargo', 'makepkg', 'git clone'):
            self.assertNotIn(text, (ROOT / 'nxpm/nxpm').read_text())


if __name__ == '__main__':
    unittest.main()
