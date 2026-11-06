import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import shutil
import ssl
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('gnome_sender', ROOT / 'gnome/lib/localsend-send.py')
sender = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sender)


class MigrationTests(unittest.TestCase):

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
            settings_output = Path(scratch) / 'settings'
            subprocess.run(['bash', '-c', 'source "$1"; srcdir="$2"; pkgdir="$3"; package',
                            'stage', str(ROOT / 'packaging/nodalix-settings/PKGBUILD'),
                            str(ROOT), str(settings_output)], check=True)
            login_background = settings_output / 'usr/share/gnome-shell/extensions/nodalix-login-background@getnodalia.com'
            self.assertFalse(login_background.is_symlink())
            self.assertTrue(login_background.is_dir())
            self.assertTrue((login_background / 'extension.js').is_file())
            self.assertTrue((login_background / 'metadata.json').is_file())
            profile=json.loads((ROOT/'gnome/session/extensions.json').read_text())
            for uuid in profile:
                if uuid.startswith('hanabi'):continue  # separate video package
                # Real directories only: pacman cannot turn a 0.4.x directory into a symlink.
                directory = f'usr/share/gnome-shell/extensions/{uuid}'
                self.assertFalse((output / directory).is_symlink())
                self.assertIn(f'{directory}/extension.js', package_paths[1])
                self.assertIn(f'{directory}/metadata.json', package_paths[1])
                self.assertEqual((output/f'usr/share/gnome-shell/extensions/{uuid}/metadata.json').stat().st_mode & 0o777,0o644)
            self.assertFalse(any('/hypr/' in path or '/quickshell/' in path for paths in package_paths for path in paths))


class Response:
    def __init__(self, code=200, body=b''):
        self.status, self.body = code, body

    def read(self):
        return self.body


class SenderTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('openssl'), 'OpenSSL is needed to generate a disposable certificate')
    def test_tls_fingerprint_is_checked_before_sending_http(self):
        requests = []

        class Receiver(http.server.BaseHTTPRequestHandler):
            def handle_one_request(self):
                try:
                    super().handle_one_request()
                except BrokenPipeError:
                    # A rejected fingerprint closes during the TLS handshake.
                    self.close_connection = True

            def do_HEAD(self):
                requests.append(self.path)
                self.send_response(204)
                self.end_headers()

            def log_message(self, *args):
                pass

        with tempfile.TemporaryDirectory() as directory:
            certificate = Path(directory) / 'certificate.pem'
            key = Path(directory) / 'key.pem'
            subprocess.run(
                ['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
                 '-subj', '/CN=LocalSend Test', '-out', str(certificate), '-keyout', str(key)],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            context.load_cert_chain(certificate, key)
            with http.server.ThreadingHTTPServer(('127.0.0.1', 0), Receiver) as receiver:
                receiver.socket = context.wrap_socket(receiver.socket, server_side=True)
                thread = threading.Thread(target=receiver.serve_forever, daemon=True)
                thread.start()
                try:
                    peer = dict(protocol='https', ip='127.0.0.1', port=receiver.server_port,
                                fingerprint='0' * 64)
                    identity = dict(certificate=str(certificate), key=str(key))
                    with self.assertRaisesRegex(ValueError, 'identidad'):
                        sender.connection(peer, identity)
                    self.assertEqual(requests, [])
                    der = ssl.PEM_cert_to_DER_cert(certificate.read_text())
                    peer['fingerprint'] = hashlib.sha256(der).hexdigest()
                    client = sender.connection(peer, identity)
                    try:
                        client.request('HEAD', '/probe')
                        self.assertEqual(client.getresponse().status, 204)
                    finally:
                        client.close()
                    self.assertEqual(requests, ['/probe'])
                finally:
                    receiver.shutdown()
                    thread.join(timeout=5)

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

class ExtensionProfileTests(unittest.TestCase):
    def test_profile_defaults_match_packaged_schema_types(self):
        from gi.repository import Gio,GLib
        with tempfile.TemporaryDirectory() as scratch:
            for row in json.loads((ROOT/'gnome/session/extension-defaults.json').read_text()):
                directory=Path(scratch)/row['uuid'];directory.mkdir(exist_ok=True)
                for source in (ROOT/'gnome/extensions'/row['uuid']/'schemas').glob('*.xml'):
                    (directory/source.name).write_bytes(source.read_bytes())
                subprocess.run(['glib-compile-schemas',str(directory)],check=True)
                schemas=Gio.SettingsSchemaSource.new_from_directory(str(directory),Gio.SettingsSchemaSource.get_default(),False)
                schema=schemas.lookup(row['schema'],False)
                self.assertIsNotNone(schema)
                for key,text in row['values'].items():
                    value=GLib.Variant.parse(None,text,None,None)
                    self.assertEqual(value.get_type_string(),schema.get_key(key).get_value_type().dup_string(),key)
                    self.assertTrue(schema.get_key(key).range_check(value),key)
