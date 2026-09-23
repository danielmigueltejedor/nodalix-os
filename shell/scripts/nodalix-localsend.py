#!/usr/bin/env python3
"""Small LocalSend v2 bridge for the Nodalix control centre.

Uses only the Python standard library.  The LAN API listens on LocalSend's
standard TCP/UDP port.  A separate loopback-only API is consumed by QML.
"""

from __future__ import annotations

import hashlib
import http
import http.client
import ipaddress
import json
import mimetypes
import os
import shutil
import socket
import ssl
import subprocess
import threading
import time
import urllib.parse
import urllib.error
import urllib.request
import uuid
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LAN_PORT = 53317
CONTROL_PORT = 53318
MULTICAST_GROUP = "224.0.0.167"
PROTOCOL_VERSION = "2.2"


def xdg_user_dir(kind: str, fallback: str) -> Path:
    """Resolve an XDG user directory, with a localized safe fallback."""
    try:
        value = subprocess.check_output(
            ["xdg-user-dir", kind], text=True, stderr=subprocess.DEVNULL
        ).strip()
        if value:
            return Path(value)
    except (OSError, subprocess.SubprocessError):
        pass
    return Path.home() / fallback


class Bridge:
    def __init__(self) -> None:
        self.lock = threading.RLock()
        self.stop = threading.Event()
        self.alias = socket.gethostname() or "Nodalix"
        self.state_dir = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "nodalix-localsend"
        self.destination = xdg_user_dir("DOWNLOAD", "Descargas") / "LocalSend"
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.destination.mkdir(parents=True, exist_ok=True)
        self.settings_path = self.state_dir / "settings.json"
        self.favorites: dict[str, str] = {}
        self._load_settings()
        self.cert, self.key, self.fingerprint = self._identity()
        self._client_ssl_context = self._make_ssl_context()
        self.peers: dict[str, dict] = {}
        self.pending: dict[str, dict] = {}
        self.sessions: dict[str, dict] = {}
        self.transfer = {"state": "idle", "message": ""}

    def _load_settings(self) -> None:
        try:
            value = json.loads(self.settings_path.read_text())
            alias = str(value.get("alias") or "").strip()
            if alias:
                self.alias = alias[:64]
            favorites = value.get("favorites") or {}
            if isinstance(favorites, dict):
                self.favorites = {self.normalize_fingerprint(str(k)): str(v)
                                  for k, v in favorites.items() if self.normalize_fingerprint(str(k))}
        except (FileNotFoundError, json.JSONDecodeError, OSError, TypeError):
            pass

    def _save_settings(self) -> None:
        temp = self.settings_path.with_suffix(".tmp")
        temp.write_text(json.dumps({"alias": self.alias, "favorites": self.favorites}, ensure_ascii=False))
        temp.chmod(0o600)
        temp.replace(self.settings_path)

    def set_alias(self, alias: str) -> bool:
        value = " ".join(alias.strip().split())[:64]
        if not value:
            return False
        with self.lock:
            self.alias = value
            self._save_settings()
        threading.Thread(target=self.announce, daemon=True).start()
        return True

    def set_favorite(self, fingerprint: str, favorite: bool, alias: str = "") -> bool:
        fingerprint = self.normalize_fingerprint(fingerprint)
        if not fingerprint:
            return False
        with self.lock:
            if favorite:
                peer = self.peers.get(fingerprint, {})
                self.favorites[fingerprint] = alias or str(peer.get("alias") or fingerprint[:12])
            else:
                self.favorites.pop(fingerprint, None)

            # Keep the discovered peer state in sync immediately. Otherwise
            # /status.devices keeps exposing the old favorite value until the
            # device is rediscovered.
            peer = self.peers.get(fingerprint)
            if peer is not None:
                peer["favorite"] = favorite

            self._save_settings()
        return True

    @staticmethod
    def normalize_fingerprint(value: str) -> str:
        return "".join(ch for ch in value.upper() if ch.isalnum())

    @staticmethod
    def lan_ip() -> str:
        """Return the IPv4 address selected by the kernel's default LAN route."""
        probe = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            probe.connect(("224.0.0.167", LAN_PORT))
            return str(probe.getsockname()[0])
        finally:
            probe.close()

    def _identity(self) -> tuple[Path, Path, str]:
        cert = self.state_dir / "identity.crt"
        key = self.state_dir / "identity.key"
        if not cert.exists() or not key.exists():
            subprocess.run([
                "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", str(key), "-out", str(cert), "-days", "3650",
                "-subj", "/CN=LocalSend User",
            ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            key.chmod(0o600)
        der = ssl.PEM_cert_to_DER_cert(cert.read_text())
        return cert, key, hashlib.sha256(der).hexdigest().upper()

    def info(self, announce: bool | None = None) -> dict:
        value = {
            "alias": self.alias,
            "version": PROTOCOL_VERSION,
            "deviceModel": "Nodalix",
            "deviceType": "desktop",
            "fingerprint": self.fingerprint,
            "port": LAN_PORT,
            "protocol": "https",
            "download": False,
        }
        if announce is not None:
            value["announce"] = announce
        return value

    def response_info(self) -> dict:
        value = self.info()
        value.pop("port", None)
        value.pop("protocol", None)
        return value

    def add_peer(self, ip: str, info: dict) -> None:
        fp = self.normalize_fingerprint(str(info.get("fingerprint", "")))
        if not fp or fp == self.fingerprint:
            return
        with self.lock:
            self.peers[fp] = {
                "fingerprint": fp,
                "alias": str(info.get("alias") or ip),
                "deviceModel": str(info.get("deviceModel") or ""),
                "deviceType": str(info.get("deviceType") or "desktop"),
                "ip": ip,
                "port": int(info.get("port") or LAN_PORT),
                "protocol": str(info.get("protocol") or "https"),
                "seen": time.time(),
                "favorite": fp in self.favorites,
            }

    def peer_is_recent(self, ip: str, max_age: float = 45) -> bool:
        """Avoid probing peers already refreshed by multicast or registration."""
        now = time.time()
        with self.lock:
            return any(peer.get("ip") == ip and now - peer.get("seen", 0) < max_age
                       for peer in self.peers.values())

    def public_status(self) -> dict:
        now = time.time()
        with self.lock:
            # A receiver announces every few seconds while it is actually
            # available.  Keeping old entries for too long made Nautilus show
            # iPhones whose LocalSend listener had already been suspended.
            peers = [p.copy() for p in self.peers.values() if now - p["seen"] < 40]
            peers.sort(key=lambda p: p["alias"].lower())
            pending = []
            for p in self.pending.values():
                pending.append({
                    "id": p["id"], "alias": p["alias"],
                    "files": [{"name": f.get("fileName", ""), "size": f.get("size", 0)} for f in p["files"].values()],
                    "total": sum(int(f.get("size", 0)) for f in p["files"].values()),
                })
            favorites = [{"fingerprint": fp, "alias": alias}
                         for fp, alias in sorted(self.favorites.items(), key=lambda item: item[1].lower())]
            return {"enabled": True, "alias": self.alias, "devices": peers,
                    "favorites": favorites, "pending": pending, "transfer": self.transfer.copy()}

    def _make_ssl_context(self) -> ssl.SSLContext:
        ctx = ssl.create_default_context()
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
        ctx.load_cert_chain(str(self.cert), str(self.key))
        return ctx

    def ssl_context(self) -> ssl.SSLContext:
        return self._client_ssl_context

    def request_json(self, peer: dict, path: str, payload: dict, timeout: int = 65) -> tuple[int, dict]:
        host = peer["ip"]
        if ":" in host and not host.startswith("["):
            host = f"[{host}]"
        url = f"{peer['protocol']}://{host}:{peer['port']}{path}"
        request = urllib.request.Request(url, data=json.dumps(payload).encode(),
                                         headers={"Content-Type": "application/json"}, method="POST")
        kwargs = {"timeout": timeout}
        if peer["protocol"] == "https":
            kwargs["context"] = self.ssl_context()
        try:
            with urllib.request.urlopen(request, **kwargs) as response:
                raw = response.read()
                return response.status, json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            detail = error.read(4096).decode("utf-8", "replace").strip()
            labels = {
                401: "el receptor requiere un PIN o el PIN no es válido",
                403: "el receptor rechazó la transferencia",
                409: "el receptor está ocupado con otra transferencia",
                429: "el receptor ha limitado temporalmente los envíos",
            }
            message = labels.get(error.code, f"respuesta HTTP {error.code} del receptor")
            raise RuntimeError(message + (f": {detail}" if detail else "")) from error

    def register_with(self, peer: dict) -> None:
        try:
            _, response = self.request_json(peer, "/api/localsend/v2/register", self.info(), 4)
            self.add_peer(peer["ip"], {**response, "fingerprint": peer["fingerprint"],
                                      "port": peer["port"], "protocol": peer["protocol"]})
        except Exception:
            pass

    def probe_host(self, ip: str) -> None:
        if ip == self.lan_ip() or self.peer_is_recent(ip):
            return
        port_probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        port_probe.settimeout(.12)
        try:
            if port_probe.connect_ex((ip, LAN_PORT)) != 0:
                return
        finally:
            port_probe.close()
        for protocol in ("https", "http"):
            try:
                url = f"{protocol}://{ip}:{LAN_PORT}/api/localsend/v2/info"
                kwargs = {"timeout": .35}
                if protocol == "https":
                    kwargs["context"] = self.ssl_context()
                with urllib.request.urlopen(url, **kwargs) as response:
                    info = json.loads(response.read())
                info["protocol"] = protocol
                info["port"] = int(info.get("port") or LAN_PORT)
                self.add_peer(ip, info)
                peer = {"ip": ip, "port": info["port"], "protocol": protocol,
                        "fingerprint": str(info.get("fingerprint") or "")}
                self.register_with(peer)
                return
            except Exception:
                continue

    def scan_lan(self) -> None:
        network = ipaddress.ip_network(f"{self.lan_ip()}/24", strict=False)
        with ThreadPoolExecutor(max_workers=32, thread_name_prefix="localsend-scan") as pool:
            list(pool.map(lambda address: self.probe_host(str(address)), network.hosts()))

    def scanner_loop(self) -> None:
        while not self.stop.is_set():
            try:
                self.scan_lan()
            except OSError:
                pass
            # Multicast is the primary discovery path. The subnet sweep is a
            # fallback for devices whose multicast is filtered, so running it
            # every two minutes is sufficient and avoids needless TLS traffic.
            self.stop.wait(120)

    def announce(self, announce: bool = True) -> None:
        payload = json.dumps(self.info(announce)).encode()
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 1)
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF,
                        socket.inet_aton(self.lan_ip()))
        try:
            for delay in (0, .1, .5):
                if delay:
                    time.sleep(delay)
                sock.sendto(payload, (MULTICAST_GROUP, LAN_PORT))
        finally:
            sock.close()

    def multicast_loop(self) -> None:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("", LAN_PORT))
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP,
                        socket.inet_aton(MULTICAST_GROUP) + socket.inet_aton(self.lan_ip()))
        sock.settimeout(1)
        while not self.stop.is_set():
            try:
                data, source = sock.recvfrom(65535)
                info = json.loads(data.decode())
                fp = self.normalize_fingerprint(str(info.get("fingerprint") or ""))
                with self.lock:
                    existing = self.peers.get(fp, {}).copy()
                needs_registration = bool(fp) and (
                    not existing
                    or existing.get("ip") != source[0]
                    or existing.get("port") != int(info.get("port") or LAN_PORT)
                    or existing.get("protocol") != str(info.get("protocol") or "https")
                )
                self.add_peer(source[0], info)
                if info.get("announce") and info.get("fingerprint") != self.fingerprint:
                    # Multicast response is the protocol fallback when the peer
                    # cannot register over HTTP/TLS (common on mobile devices).
                    threading.Thread(target=self.announce, args=(False,), daemon=True).start()
                    if needs_registration:
                        peer = {"ip": source[0], "port": int(info.get("port") or LAN_PORT),
                                "protocol": str(info.get("protocol") or "https"),
                                "fingerprint": str(info.get("fingerprint") or "")}
                        threading.Thread(target=self.register_with, args=(peer,), daemon=True).start()
            except (socket.timeout, json.JSONDecodeError, UnicodeDecodeError):
                continue
            except OSError:
                break
        sock.close()

    def announcer_loop(self) -> None:
        while not self.stop.is_set():
            try:
                self.announce()
            except OSError:
                pass
            self.stop.wait(15)

    @staticmethod
    def safe_name(name: str) -> str:
        name = Path(name.replace("\\", "/")).name.strip().replace("\x00", "")
        return name or "received-file"

    @staticmethod
    def safe_relative_name(name: str) -> Path:
        """Keep LocalSend folder structure without allowing path traversal."""
        parts = []
        for part in name.replace("\\", "/").split("/"):
            value = part.strip().replace("\x00", "")
            if value and value not in (".", ".."):
                parts.append(value)
        return Path(*parts) if parts else Path("received-file")

    def destination_for(self, name: str) -> Path:
        base = self.destination / self.safe_relative_name(name)
        base.parent.mkdir(parents=True, exist_ok=True)
        if not base.exists():
            return base
        stem, suffix = base.stem, base.suffix
        for n in range(1, 10000):
            candidate = base.with_name(f"{stem} ({n}){suffix}")
            if not candidate.exists():
                return candidate
        raise RuntimeError("No free destination name")

    def decide(self, request_id: str, accept: bool) -> bool:
        with self.lock:
            item = self.pending.get(request_id)
            if not item:
                return False
            item["accepted"] = accept
            item["event"].set()
            return True

    def send_async(self, fingerprint: str, paths: list[str]) -> None:
        threading.Thread(target=self._send, args=(fingerprint, paths), daemon=True).start()

    def upload_file(self, peer: dict, upload_path: str, source: Path) -> None:
        """Stream one file directly from disk with a fixed content length."""
        host = peer["ip"]
        port = peer["port"]
        if peer["protocol"] == "https":
            connection = http.client.HTTPSConnection(
                host, port, timeout=300, context=self.ssl_context()
            )
        else:
            connection = http.client.HTTPConnection(host, port, timeout=300)
        try:
            connection.putrequest("POST", upload_path)
            connection.putheader("Content-Type", "application/octet-stream")
            connection.putheader("Content-Length", str(source.stat().st_size))
            connection.endheaders()
            with source.open("rb") as stream:
                for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                    connection.send(chunk)
            response = connection.getresponse()
            detail = response.read(4096).decode("utf-8", "replace").strip()
            if response.status < 200 or response.status >= 300:
                raise RuntimeError(
                    f"El receptor rechazó {source.name} ({response.status})"
                    + (f": {detail}" if detail else "")
                )
        finally:
            connection.close()

    def _send(self, fingerprint: str, paths: list[str]) -> dict:
        started = time.monotonic()
        fingerprint = self.normalize_fingerprint(fingerprint)
        try:
            with self.lock:
                peer = self.peers.get(fingerprint, {}).copy()
                self.transfer = {"state": "preparing", "message": peer.get("alias", "")}
            if not peer:
                raise RuntimeError("El dispositivo ya no está disponible")
            files, sources = {}, {}
            for raw in paths:
                path = Path(raw).expanduser().resolve()
                candidates: list[tuple[Path, str]] = []
                if path.is_file():
                    candidates.append((path, path.name))
                elif path.is_dir():
                    for child in sorted(path.rglob("*")):
                        if child.is_file() and not child.is_symlink():
                            candidates.append((child, child.relative_to(path.parent).as_posix()))
                for source, transfer_name in candidates:
                    file_id = uuid.uuid4().hex
                    files[file_id] = {
                        "id": file_id,
                        "fileName": transfer_name,
                        "size": source.stat().st_size,
                        "fileType": mimetypes.guess_type(source.name)[0] or "application/octet-stream",
                        # LocalSend defines sha256 as nullable. Omitting the
                        # preliminary full-file hash removes the long pause
                        # before the receiver even sees the request.
                        "sha256": None,
                    }
                    sources[file_id] = source
            if not files:
                raise RuntimeError("No se seleccionó ningún archivo válido")
            metadata_ms = round((time.monotonic() - started) * 1000)
            with self.lock:
                self.transfer = {
                    "state": "connecting",
                    "message": f"Conectando con {peer['alias']}",
                    "files": len(files),
                    "bytes": sum(int(item["size"]) for item in files.values()),
                    "metadataMs": metadata_ms,
                }
            payload = {"info": self.info(), "files": files}
            try:
                status, response = self.request_json(
                    peer, "/api/localsend/v2/prepare-upload", payload
                )
            except urllib.error.URLError as first_error:
                # The peer may have changed IP since its last multicast packet.
                # Rediscover once before reporting failure instead of sending to
                # a stale address cached by the file-manager menu.
                with self.lock:
                    self.peers.pop(fingerprint, None)
                try:
                    self.announce()
                    time.sleep(.25)
                    self.scan_lan()
                except OSError:
                    pass
                with self.lock:
                    refreshed = self.peers.get(fingerprint, {}).copy()
                if not refreshed:
                    hint = (
                        " Abre LocalSend en el iPhone y manténlo en primer plano."
                        if str(peer.get("deviceType")) == "mobile" else ""
                    )
                    raise RuntimeError(
                        f"{peer.get('alias', 'El dispositivo')} ya no está disponible.{hint}"
                    ) from first_error
                peer = refreshed
                status, response = self.request_json(
                    peer, "/api/localsend/v2/prepare-upload", payload
                )
            if status == 204:
                with self.lock:
                    self.transfer = {"state": "done", "message": f"No había nada que enviar a {peer['alias']}"}
                    result = self.transfer.copy()
                print(f"LocalSend: {result['message']}", flush=True)
                return result
            session_id = response["sessionId"]
            accepted = list(response.get("files", {}).items())
            if not accepted:
                raise RuntimeError("El receptor no aceptó ningún archivo")
            prepare_ms = round((time.monotonic() - started) * 1000) - metadata_ms
            with self.lock:
                self.transfer = {
                    "state": "sending",
                    "message": f"Enviando a {peer['alias']}",
                    "files": len(accepted),
                    "metadataMs": metadata_ms,
                    "receiverMs": prepare_ms,
                }

            def upload(item: tuple[str, str]) -> None:
                file_id, token = item
                source = sources[file_id]
                query = urllib.parse.urlencode({"sessionId": session_id, "fileId": file_id, "token": token})
                self.upload_file(peer, f"/api/localsend/v2/upload?{query}", source)

            # The protocol explicitly allows concurrent uploads. A conservative
            # limit accelerates folders without overwhelming phones or Wi-Fi.
            with ThreadPoolExecutor(max_workers=min(3, len(accepted)), thread_name_prefix="localsend-upload") as pool:
                list(pool.map(upload, accepted))
            with self.lock:
                self.transfer = {
                    "state": "done",
                    "message": f"Enviado a {peer['alias']}",
                    "durationMs": round((time.monotonic() - started) * 1000),
                    "metadataMs": metadata_ms,
                    "receiverMs": prepare_ms,
                }
                result = self.transfer.copy()
            print(
                f"LocalSend: enviado a {peer['alias']} ({result['durationMs']} ms; "
                f"receptor {result['receiverMs']} ms)",
                flush=True,
            )
            return result
        except Exception as error:
            if isinstance(error, urllib.error.URLError):
                with self.lock:
                    self.peers.pop(fingerprint, None)
            with self.lock:
                self.transfer = {
                    "state": "error",
                    "message": str(error),
                    "durationMs": round((time.monotonic() - started) * 1000),
                }
                result = self.transfer.copy()
            print(f"LocalSend: error al enviar: {result['message']}", flush=True)
            return result


BRIDGE = Bridge()


class Handler(BaseHTTPRequestHandler):
    server_version = "NodalixLocalSend/1"

    def log_message(self, *_args) -> None:
        pass

    def body(self) -> bytes:
        return self.rfile.read(int(self.headers.get("Content-Length", "0")))

    def body_chunks(self):
        if "chunked" in self.headers.get("Transfer-Encoding", "").lower():
            while True:
                line = self.rfile.readline().strip()
                if not line:
                    continue
                size = int(line.split(b";", 1)[0], 16)
                if size == 0:
                    while self.rfile.readline().strip():
                        pass
                    return
                remaining = size
                while remaining:
                    chunk = self.rfile.read(min(1024 * 1024, remaining))
                    if not chunk:
                        raise ConnectionError("Incomplete chunked upload")
                    remaining -= len(chunk)
                    yield chunk
                self.rfile.read(2)
            return
        remaining = int(self.headers.get("Content-Length", "0"))
        while remaining:
            chunk = self.rfile.read(min(1024 * 1024, remaining))
            if not chunk:
                raise ConnectionError("Incomplete upload")
            remaining -= len(chunk)
            yield chunk

    def json_body(self) -> dict:
        raw = self.body()
        return json.loads(raw) if raw else {}

    def reply(self, code: int, value=None) -> None:
        if code >= 400 and value is None:
            value = {"message": http.HTTPStatus(code).phrase}
        data = b"" if value is None else json.dumps(value).encode()
        self.send_response(code)
        if value is not None:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if data:
            self.wfile.write(data)

    @property
    def control(self) -> bool:
        return self.server.server_address[1] == CONTROL_PORT

    def do_GET(self) -> None:
        path = urllib.parse.urlsplit(self.path).path
        if self.control and path == "/status":
            self.reply(200, BRIDGE.public_status())
        elif not self.control and path == "/api/localsend/v2/info":
            self.reply(200, BRIDGE.response_info())
        else:
            self.reply(404)

    def do_POST(self) -> None:
        try:
            path = urllib.parse.urlsplit(self.path).path
            if self.control:
                self.control_post(path)
            else:
                self.lan_post(path)
        except (json.JSONDecodeError, ValueError) as error:
            self.reply(400, {"message": f"Invalid request: {error}"})
        except (BrokenPipeError, ConnectionResetError):
            pass
        except Exception as error:
            self.reply(500, {"message": str(error)})

    def control_post(self, path: str) -> None:
        data = self.json_body()
        if path == "/announce":
            def discover():
                try:
                    BRIDGE.announce()
                finally:
                    BRIDGE.scan_lan()

            threading.Thread(target=discover, daemon=True).start()
            self.reply(204)
        elif path in ("/accept", "/decline"):
            self.reply(204 if BRIDGE.decide(str(data.get("id", "")), path == "/accept") else 404)
        elif path == "/send":
            fingerprint = str(data.get("fingerprint", ""))
            paths = [str(p) for p in data.get("paths", [])]
            if data.get("wait"):
                self.reply(200, BRIDGE._send(fingerprint, paths))
            else:
                BRIDGE.send_async(fingerprint, paths)
                self.reply(202)
        elif path == "/alias":
            self.reply(204 if BRIDGE.set_alias(str(data.get("alias", ""))) else 400)
        elif path == "/favorite":
            ok = BRIDGE.set_favorite(str(data.get("fingerprint", "")), bool(data.get("favorite")),
                                     str(data.get("alias", "")))
            self.reply(204 if ok else 400)
        else:
            self.reply(404)

    def lan_post(self, path: str) -> None:
        if path == "/api/localsend/v2/register":
            data = self.json_body()
            BRIDGE.add_peer(self.client_address[0], data)
            self.reply(200, BRIDGE.response_info())
            return
        if path == "/api/localsend/v2/prepare-upload":
            data = self.json_body()
            files = data.get("files") or {}
            sender = data.get("info") or {}
            sender_fp = BRIDGE.normalize_fingerprint(str(sender.get("fingerprint") or ""))
            sender_alias = str(sender.get("alias") or self.client_address[0])
            trusted = sender_fp in BRIDGE.favorites
            print(
                f"[LocalSend RX] prepare-upload alias={sender_alias!r} "
                f"fingerprint={sender_fp!r} trusted={trusted} files={len(files)}",
                flush=True,
            )
            request_id = uuid.uuid4().hex
            pending = {"id": request_id, "alias": sender_alias, "fingerprint": sender_fp,
                       "files": files, "event": threading.Event(), "accepted": trusted}
            if not trusted:
                with BRIDGE.lock:
                    BRIDGE.pending[request_id] = pending
                pending["event"].wait(60)
                with BRIDGE.lock:
                    BRIDGE.pending.pop(request_id, None)
            if not pending["accepted"]:
                self.reply(403)
                return
            session_id = uuid.uuid4().hex
            tokens = {file_id: uuid.uuid4().hex for file_id in files}
            with BRIDGE.lock:
                BRIDGE.sessions[session_id] = {"files": files, "tokens": tokens, "received": []}
                BRIDGE.transfer = {"state": "receiving", "message": pending["alias"]}
            self.reply(200, {"sessionId": session_id, "files": tokens})
            return
        if path == "/api/localsend/v2/upload":
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
            session_id = (query.get("sessionId") or [""])[0]
            file_id = (query.get("fileId") or [""])[0]
            token = (query.get("token") or [""])[0]
            with BRIDGE.lock:
                session = BRIDGE.sessions.get(session_id)
            if not session or session["tokens"].get(file_id) != token:
                self.reply(403)
                return
            meta = session["files"].get(file_id) or {}
            target = BRIDGE.destination_for(str(meta.get("fileName") or "received-file"))
            temp = target.with_name(target.name + ".part")
            digest = hashlib.sha256()
            received = 0
            with temp.open("wb") as output:
                for chunk in self.body_chunks():
                    output.write(chunk)
                    digest.update(chunk)
                    received += len(chunk)
            expected = str(meta.get("sha256") or "").lower()
            expected_size = int(meta.get("size") or 0)
            if (expected_size and received != expected_size) or (expected and digest.hexdigest().lower() != expected):
                temp.unlink(missing_ok=True); self.reply(422); return
            shutil.move(temp, target)
            with BRIDGE.lock:
                session["received"].append(file_id)
                if len(session["received"]) >= len(session["files"]):
                    BRIDGE.transfer = {
                        "state": "done",
                        "direction": "receive",
                        "message": "Archivo recibido correctamente",
                        "eventId": uuid.uuid4().hex,
                        "completedAt": time.time(),
                    }
                    BRIDGE.sessions.pop(session_id, None)
            self.reply(200)
            return
        if path == "/api/localsend/v2/cancel":
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
            with BRIDGE.lock:
                BRIDGE.sessions.pop((query.get("sessionId") or [""])[0], None)
                BRIDGE.transfer = {"state": "idle", "message": "Transferencia cancelada"}
            self.reply(204)
            return
        self.reply(404)


def main() -> None:
    lan = ThreadingHTTPServer(("0.0.0.0", LAN_PORT), Handler)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(str(BRIDGE.cert), str(BRIDGE.key))
    lan.socket = tls.wrap_socket(lan.socket, server_side=True)
    control = ThreadingHTTPServer(("127.0.0.1", CONTROL_PORT), Handler)
    threading.Thread(target=BRIDGE.multicast_loop, daemon=True).start()
    threading.Thread(target=BRIDGE.announcer_loop, daemon=True).start()
    threading.Thread(target=BRIDGE.scanner_loop, daemon=True).start()
    threading.Thread(target=control.serve_forever, daemon=True).start()
    try:
        lan.serve_forever()
    finally:
        BRIDGE.stop.set(); lan.server_close(); control.shutdown(); control.server_close()


if __name__ == "__main__":
    main()
