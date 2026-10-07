#!/usr/bin/env python3
"""Detached, streaming LocalSend sender using GLocalSend's discovery/identity."""
from __future__ import annotations

import argparse
import hashlib
import http.client
import ipaddress
import json
import mimetypes
from pathlib import Path
import ssl
import subprocess
import tempfile
import urllib.parse
import uuid
import zipfile

from gi.repository import Gio, GLib

BUS = 'com.nodalix.LocalSend'
OBJECT = '/com/nodalix/LocalSend'
IFACE = 'com.nodalix.LocalSend1'


def status():
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    result = bus.call_sync(BUS, OBJECT, IFACE, 'GetStatus', None,
                           GLib.VariantType.new('(s)'), Gio.DBusCallFlags.NONE, 1500, None)
    return json.loads(result.unpack()[0])


def notify(title, body):
    try:
        subprocess.run(['notify-send', '--app-name=Nodalix LocalSend', '--icon=send-to-symbolic',
                        title, body], check=False, timeout=5)
    except (OSError, subprocess.SubprocessError):
        pass


def fingerprint(value):
    return ''.join(c for c in str(value).upper() if c.isalnum())


def connection(peer, identity):
    # Use a literal discovered address rather than allowing URL credentials,
    # paths, or redirect targets to reach unrelated hosts.
    address = str(ipaddress.ip_address(peer['ip']))
    port = int(peer['port'])
    if not 0 < port <= 65535:
        raise ValueError('Puerto LocalSend inválido')
    protocol = peer.get('protocol')
    if protocol == 'https':
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
        context.load_cert_chain(identity['certificate'], identity['key'])
        client = http.client.HTTPSConnection(address, port, context=context, timeout=120)
        client.connect()
        expected = fingerprint(peer['fingerprint'])
        actual = hashlib.sha256(client.sock.getpeercert(binary_form=True)).hexdigest().upper()
        if expected != actual:
            client.close()
            raise ValueError('La identidad del receptor LocalSend ha cambiado')
        return client
    if protocol == 'http':
        return http.client.HTTPConnection(address, port, timeout=120)
    raise ValueError('Protocolo LocalSend no compatible')


def prepare_files(paths, scratch):
    result = {}
    for value in paths:
        source = Path(value).absolute()
        if not source.exists():
            raise ValueError(f'No existe: {source.name}')
        if source.is_dir():
            zipped = Path(scratch) / (uuid.uuid4().hex + '.zip')
            with zipfile.ZipFile(zipped, 'w', zipfile.ZIP_DEFLATED) as archive:
                for child in sorted(source.rglob('*')):
                    if child.is_file() and not child.is_symlink():
                        archive.write(child, str(Path(source.name) / child.relative_to(source)))
            path, name = zipped, source.name + '.zip'
        elif source.is_file():
            path, name = source, source.name
        else:
            raise ValueError('Selecciona archivos o carpetas normales')
        key = uuid.uuid4().hex
        result[key] = (path, {'id': key, 'fileName': name, 'size': path.stat().st_size,
                              'fileType': mimetypes.guess_type(name)[0] or 'application/octet-stream'})
    return result


def send(peer, identity, files):
    data = json.dumps({'info': identity['identity'], 'files': {k: v[1] for k, v in files.items()}}).encode()
    pin = None
    for attempt in range(4):
        client = connection(peer, identity)
        try:
            query = '?' + urllib.parse.urlencode({'pin': pin}) if pin is not None else ''
            client.request('POST', '/api/localsend/v2/prepare-upload' + query, data,
                           {'Content-Type': 'application/json'})
            response = client.getresponse()
            code, body = response.status, response.read()
        finally:
            client.close()
        if code != 401:
            break
        entered = subprocess.run(['zenity', '--password', '--title=PIN de LocalSend'],
                                 capture_output=True, text=True, check=False)
        if entered.returncode:
            raise ValueError('Envío cancelado')
        pin = entered.stdout.strip()
    if code == 204:
        return
    if code != 200:
        raise ValueError(f'El receptor rechazó el envío (HTTP {code})')
    session = json.loads(body)
    if not isinstance(session, dict) or not isinstance(session.get('sessionId'), str):
        raise ValueError('Respuesta LocalSend inválida')
    accepted = session.get('files', {})
    if not isinstance(accepted, dict) or not accepted:
        raise ValueError('El receptor no aceptó ningún archivo')
    for file_id, token in accepted.items():
        if file_id not in files or not isinstance(token, str):
            raise ValueError('Respuesta LocalSend inválida')
        path, metadata = files[file_id]
        query = urllib.parse.urlencode({'sessionId': session['sessionId'], 'fileId': file_id, 'token': token})
        client = connection(peer, identity)
        try:
            with path.open('rb') as source:
                client.request('POST', '/api/localsend/v2/upload?' + query, source,
                               {'Content-Type': 'application/octet-stream', 'Content-Length': str(metadata['size'])})
                response = client.getresponse()
                response.read()
                if response.status != 200:
                    raise ValueError(f'No se pudo subir {metadata["fileName"]} (HTTP {response.status})')
        finally:
            client.close()
    if len(accepted) < len(files):
        raise ValueError(f'Envío parcial: {len(accepted)} de {len(files)} archivos aceptados')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--fingerprint', required=True)
    parser.add_argument('--alias', default='dispositivo')
    parser.add_argument('paths', nargs='+')
    args = parser.parse_args()
    try:
        current = status()
        if not current.get('enabled'):
            raise ValueError('Activa LocalSend en Ajustes → Compartir → LocalSend')
        peer = next((p for p in current['devices'] if fingerprint(p['fingerprint']) == fingerprint(args.fingerprint)), None)
        if peer is None:
            raise ValueError('El dispositivo ya no está disponible')
        notify('Preparando envío', f'Conectando con {peer["alias"]}')
        with tempfile.TemporaryDirectory(prefix='nodalix-send-') as scratch:
            send(peer, current, prepare_files(args.paths, scratch))
        notify('Envío completado', f'Archivos enviados a {peer["alias"]}')
        return 0
    except (OSError, ValueError, KeyError, TypeError, http.client.HTTPException,
            GLib.Error, subprocess.SubprocessError) as error:
        notify('No se pudo enviar', str(error))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
