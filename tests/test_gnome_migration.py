import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('gnome_sender', ROOT / 'gnome/lib/localsend-send.py')
sender = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sender)


class MigrationTests(unittest.TestCase):
    def test_legacy_guard_checks_colon_separated_session(self):
        guard = ROOT / 'packaging/nodalix-shell/is-hyprland-session'
        for session, expected in [('GNOME', 1), ('GNOME:GNOME', 1), ('', 1),
                                  ('Hyprland', 0), ('Nodalix:Hyprland', 0)]:
            with self.subTest(session=session):
                result = subprocess.run(['sh', str(guard)], env={**os.environ, 'XDG_CURRENT_DESKTOP': session})
                self.assertEqual(result.returncode, expected)

    def test_required_release_uses_gnome(self):
        data = json.loads((ROOT / 'release/components.json').read_text())
        components = data['components'] if isinstance(data, dict) else data
        required = {c['id'] for c in components if c.get('required')}
        self.assertTrue({'gnome', 'integrations'} <= required)
        self.assertFalse({'shell', 'hymission', 'greeter-theme', 'wallpaper-engine'} & required)

    def test_native_packages_stage_without_file_overlap(self):
        with tempfile.TemporaryDirectory() as scratch:
            package_paths = []
            for name in ['integrations', 'gnome']:
                output = Path(scratch) / name
                subprocess.run(['bash', '-c', 'source "$1"; srcdir="$2"; pkgdir="$3"; package',
                                'stage', str(ROOT / f'packaging/nodalix-{name}/PKGBUILD'),
                                str(ROOT), str(output)], check=True)
                package_paths.append({str(p.relative_to(output)) for p in output.rglob('*') if p.is_file() or p.is_symlink()})
            self.assertFalse(package_paths[0] & package_paths[1])
            self.assertIn('usr/lib/systemd/user/nodalix-icloud-drive.service', package_paths[0])
            self.assertNotIn('usr/lib/systemd/user/default.target.wants/nodalix-localsend.service', package_paths[0])
            self.assertIn('usr/lib/nodalix/localsend-send', package_paths[1])
            self.assertFalse(any('glocalsend@' in path for path in package_paths[1]))
            self.assertFalse(any('/hypr/' in path or '/quickshell/' in path for paths in package_paths for path in paths))


class Response:
    def __init__(self, code=200, body=b''):
        self.status, self.body = code, body

    def read(self):
        return self.body


class SenderTests(unittest.TestCase):
    def test_streams_file_and_reports_partial_acceptance(self):
        with tempfile.TemporaryDirectory() as scratch:
            paths = [Path(scratch) / name for name in ['a.txt', 'b.txt']]
            for path in paths:
                path.write_bytes(b'hello' * 10000)
            files = sender.prepare_files(paths, scratch)
            first_id = next(iter(files))
            responses = iter([Response(body=json.dumps({'sessionId': 'session', 'files': {first_id: 'token'}}).encode()), Response()])
            uploads = []

            class Client:
                def request(self, method, path, body, headers):
                    if '/upload?' in path:
                        uploads.append((body.read(), headers))
                        self.assert_stream = hasattr(body, 'read')

                def getresponse(self):
                    return next(responses)

                def close(self):
                    pass

            with patch.object(sender, 'connection', return_value=Client()):
                with self.assertRaisesRegex(ValueError, 'Envío parcial'):
                    sender.send({}, {'identity': {}}, files)
            self.assertEqual(uploads[0][0], paths[0].read_bytes())
            self.assertEqual(uploads[0][1]['Content-Length'], str(paths[0].stat().st_size))

    def test_rejects_malformed_remote_session(self):
        class Client:
            def request(self, *args):
                pass

            def getresponse(self):
                return Response(body=b'{"files":{"unknown":"token"}}')

            def close(self):
                pass

        with patch.object(sender, 'connection', return_value=Client()):
            with self.assertRaisesRegex(ValueError, 'Respuesta LocalSend inválida'):
                sender.send({}, {'identity': {}}, {})

    def test_unavailable_notifications_do_not_abort_send(self):
        with patch.object(sender.subprocess, 'run', side_effect=FileNotFoundError):
            sender.notify('title', 'body')

    def test_folder_zip_excludes_symlinked_private_files(self):
        with tempfile.TemporaryDirectory() as scratch:
            folder = Path(scratch) / 'folder'
            folder.mkdir()
            (folder / 'normal.txt').write_text('normal')
            secret = Path(scratch) / 'private.txt'
            secret.write_text('private')
            (folder / 'link.txt').symlink_to(secret)
            prepared = sender.prepare_files([folder], scratch)
            zipped = next(iter(prepared.values()))[0]
            with sender.zipfile.ZipFile(zipped) as archive:
                self.assertEqual(archive.namelist(), ['folder/normal.txt'])


if __name__ == '__main__':
    unittest.main()
