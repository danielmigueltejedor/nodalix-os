import Gio from "gi://Gio";
import GLib from "gi://GLib";
import Soup from "gi://Soup";
Gio._promisify(Soup.Session.prototype, "send_and_read_async", "send_and_read_finish");
Gio._promisify(Gio.File.prototype, "load_bytes_async", "load_bytes_finish");
import { CERT_PATH, DEFAULT_MULTICAST_GROUP, DEFAULT_PORT, DeviceType, KEY_AUTO_DISABLE_ENABLED, KEY_AUTO_DISABLE_MINUTES, KEY_PATH, PROTOCOL_VERSION, ProtocolType, decodeJson, encodeJson, ensureAlias, getDefaultDownloadFolder, sanitizeFileName, } from "./common.js";
const DEBUG_LOGGING = false;
const SOCKET_LEVEL_SOL = 1;
const SOCKET_OPTION_REUSEADDR = 2;
const SOCKET_OPTION_REUSEPORT = 15;
const HTTP_STATUS_PHRASES = {
    200: "OK",
    204: "No Content",
    400: "Bad Request",
    403: "Forbidden",
    404: "Not Found",
    409: "Conflict",
    412: "Precondition Failed",
    500: "Internal Server Error",
};
const PEER_STALE_MS = 180_000;
const REJECT_MESSAGE = "The recipient has rejected the request.";
function parseRequestUrl(message) {
    const uriString = message.get_uri().to_string();
    const schemeSeparator = uriString.indexOf("://");
    const pathStart = schemeSeparator >= 0
        ? uriString.indexOf("/", schemeSeparator + 3)
        : uriString.indexOf("/");
    const rawPath = pathStart >= 0 ? uriString.slice(pathStart) : "/";
    const queryIndex = rawPath.indexOf("?");
    const path = queryIndex >= 0 ? rawPath.slice(0, queryIndex) : rawPath;
    const queryString = queryIndex >= 0 ? rawPath.slice(queryIndex + 1) : "";
    const query = {};
    for (const pair of queryString.split("&")) {
        if (pair.trim().length === 0)
            continue;
        const [rawKey, rawValue = ""] = pair.split("=", 2);
        query[decodeURIComponent(rawKey)] = decodeURIComponent(rawValue.replace(/\+/g, " "));
    }
    return { path, query };
}
export class LocalSendService {
    _settings;
    _callbacks;
    _session;
    _server;
    _autoDisableSourceId = null;
    _alias;
    _fingerprint;
    _clientCert = null;
    _port;
    _httpPort;
    _downloadFolder;
    _peers = new Map();
    _multicastSocket = null;
    _multicastSourceId = null;
    _announcementSourceId = null;
    _peerCleanupSourceId = null;
    _incomingSession = null;
    _cancellable = null;
    constructor(settings, callbacks) {
        this._settings = settings;
        this._callbacks = callbacks;
        this._session = new Soup.Session();
        this._server = new Soup.Server();
        this._alias = ensureAlias(this._settings.get_string("alias"));
        this._fingerprint = this._settings.get_string("fingerprint");
        this._port = this._settings.get_int("port") || DEFAULT_PORT;
        this._httpPort = this._port;
        this._downloadFolder = this._resolveDownloadFolder();
        this._settings.set_string("alias", this._alias);
        this._settings.set_string("fingerprint", this._fingerprint);
        this._settings.set_int("port", this._port);
        this._settings.set_string("download-folder", this._downloadFolder);
        this._installServerHandlers();
    }
    get hasCertificate() {
        return (GLib.file_test(CERT_PATH, GLib.FileTest.EXISTS) &&
            GLib.file_test(KEY_PATH, GLib.FileTest.EXISTS));
    }
    _loadClientCertificate() {
        this._clientCert ??= Gio.TlsCertificate.new_from_files(CERT_PATH, KEY_PATH);
        this._server.tls_certificate = this._clientCert;
        return this._clientCert;
    }
    _certFingerprint() {
        return GLib.compute_checksum_for_data(GLib.ChecksumType.SHA256, this._loadClientCertificate().certificate).toUpperCase();
    }
    get enabled() {
        return this._cancellable !== null;
    }
    get peers() {
        return [...this._peers.values()]
            .filter((peer) => Date.now() - peer.lastSeenAt < PEER_STALE_MS)
            .sort((left, right) => left.alias.localeCompare(right.alias));
    }
    get httpPort() {
        return this._httpPort;
    }
    get alias() {
        return this._alias;
    }
    get fingerprint() {
        return this._fingerprint;
    }
    get port() {
        return this._httpPort;
    }
    toggleEnabled() {
        if (this.enabled) {
            this.stop();
            return;
        }
        if (!this.hasCertificate) {
            this._callbacks.onCertificateMissing();
            return;
        }
        if (this._portIsStillBound()) {
            throw new Error("LocalSend no puede volver a activarse hasta que el puerto " +
                String(this._port) +
                " is free.");
        }
        this.start();
    }
    _scheduleAutoDisable() {
        if (!this._settings.get_boolean(KEY_AUTO_DISABLE_ENABLED))
            return;
        const timeoutMinutes = Math.max(1, this._settings.get_int(KEY_AUTO_DISABLE_MINUTES) || 10);
        if (this._autoDisableSourceId !== null) {
            GLib.Source.remove(this._autoDisableSourceId);
            this._autoDisableSourceId = null;
        }
        this._autoDisableSourceId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, timeoutMinutes * 60, () => {
            this._autoDisableSourceId = null;
            this.stop();
            this._callbacks.onNotification("LocalSend desactivado", `LocalSend se ha desactivado automáticamente después de ${timeoutMinutes} minuto${timeoutMinutes === 1 ? "" : "s"}.`);
            return GLib.SOURCE_REMOVE;
        });
    }
    _cancelAutoDisable() {
        if (this._autoDisableSourceId === null)
            return;
        GLib.Source.remove(this._autoDisableSourceId);
        this._autoDisableSourceId = null;
    }
    start() {
        if (this.enabled)
            return;
        this._cancelAutoDisable();
        this._port = this._settings.get_int("port") || DEFAULT_PORT;
        this._alias = ensureAlias(this._settings.get_string("alias"));
        this._fingerprint = this._certFingerprint();
        this._downloadFolder = this._resolveDownloadFolder();
        this._settings.set_string("alias", this._alias);
        this._settings.set_string("fingerprint", this._fingerprint);
        this._settings.set_int("port", this._port);
        this._settings.set_string("download-folder", this._downloadFolder);
        this._cancellable = new Gio.Cancellable();
        this._httpPort = this._listenOnHttpPort(this._port);
        this._startDiscoverySocket();
        this._sendAnnouncement();
        this._announcementSourceId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 60, () => {
            if (!this.enabled)
                return GLib.SOURCE_REMOVE;
            this._sendAnnouncement();
            return GLib.SOURCE_CONTINUE;
        });
        this._peerCleanupSourceId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 30, () => {
            if (!this.enabled)
                return GLib.SOURCE_REMOVE;
            this._prunePeers();
            return GLib.SOURCE_CONTINUE;
        });
        this._callbacks.onStateChanged();
        this._scheduleAutoDisable();
    }
    stop() {
        this._cancelAutoDisable();
        if (!this.enabled)
            return;
        this._session.abort();
        this._cancellable?.cancel();
        this._cancellable = null;
        if (this._multicastSourceId !== null) {
            GLib.Source.remove(this._multicastSourceId);
            this._multicastSourceId = null;
        }
        if (this._multicastSocket !== null) {
            this._multicastSocket.close();
            this._multicastSocket = null;
        }
        if (this._announcementSourceId !== null) {
            GLib.Source.remove(this._announcementSourceId);
            this._announcementSourceId = null;
        }
        if (this._peerCleanupSourceId !== null) {
            GLib.Source.remove(this._peerCleanupSourceId);
            this._peerCleanupSourceId = null;
        }
        if (this._server !== null) {
            this._server.disconnect();
        }
        this._incomingSession = null;
        this._callbacks.onStateChanged();
    }
    refreshPeers() {
        if (!this.enabled)
            return;
        this._peers.clear();
        this._callbacks.onStateChanged();
        this._sendAnnouncement();
    }
    async sendFilesToPeer(peer, filePaths) {
        if (filePaths.length === 0)
            throw new Error("Selecciona al menos un archivo para enviar.");
        const items = await Promise.all(filePaths.map(async (path) => {
            const file = Gio.File.new_for_path(path);
            const info = file.query_info("standard::display-name,standard::content-type", Gio.FileQueryInfoFlags.NONE, null);
            const [bytes] = await file.load_bytes_async(this._cancellable);
            return {
                fileName: sanitizeFileName(info.get_display_name()),
                bytes: bytes.get_data() ?? new Uint8Array(),
                mimeType: info.get_content_type() ?? "application/octet-stream",
                preview: null,
            };
        }));
        await this._sendOutgoingItems(peer, items);
    }
    async sendClipboardTextToPeer(peer, text) {
        const trimmed = text.trim();
        if (trimmed.length === 0)
            throw new Error("El portapapeles está vacío.");
        await this._sendOutgoingItems(peer, [
            {
                fileName: "clipboard.txt",
                bytes: new TextEncoder().encode(trimmed),
                mimeType: "text/plain",
                preview: trimmed,
            },
        ]);
    }
    async sendClipboardImageToPeer(peer, bytes, mimeType) {
        await this._sendOutgoingItems(peer, [
            {
                fileName: `clipboard.${mimeType.split("/")[1]}`,
                bytes,
                mimeType,
                preview: null,
            },
        ]);
    }
    async sendTypedTextToPeer(peer, text) {
        const trimmed = text.trim();
        if (trimmed.length === 0)
            throw new Error("Escribe un texto antes de enviarlo.");
        await this._sendOutgoingItems(peer, [
            {
                fileName: "message.txt",
                bytes: new TextEncoder().encode(trimmed),
                mimeType: "text/plain",
                preview: trimmed,
            },
        ]);
    }
    _resolveDownloadFolder() {
        const configured = this._settings.get_string("download-folder").trim();
        if (configured.length > 0)
            return configured;
        return getDefaultDownloadFolder();
    }
    _portIsStillBound() {
        try {
            const socket = Gio.Socket.new(Gio.SocketFamily.IPV4, Gio.SocketType.STREAM, Gio.SocketProtocol.TCP);
            socket.bind(new Gio.InetSocketAddress({
                address: Gio.InetAddress.new_any(Gio.SocketFamily.IPV4),
                port: this._port,
            }), true);
            socket.close();
            return false;
        }
        catch {
            return true;
        }
    }
    _installServerHandlers() {
        this._server.add_handler("/", (_server, message) => {
            const { path } = parseRequestUrl(message);
            const method = message.get_method();
            if (method === "GET" && path === "/api/localsend/v2/info") {
                this._respondJson(message, 200, this._buildInfoPayload());
                return;
            }
            if (method === "POST" && path === "/api/localsend/v2/register") {
                this._handleRegister(message);
                return;
            }
            if (method === "POST" && path === "/api/localsend/v2/prepare-upload") {
                message.pause();
                void this._handlePrepareUpload(message).catch((error) => {
                    const messageText = error.message ?? String(error);
                    if (DEBUG_LOGGING)
                        console.warn(`Falló la preparación de la transferencia de LocalSend: ${messageText}`);
                });
                return;
            }
            if (method === "POST" && path === "/api/localsend/v2/upload") {
                this._respondUpload(message);
                return;
            }
            if (method === "POST" && path === "/api/localsend/v2/cancel") {
                this._handleCancel(message);
                return;
            }
            this._respondJson(message, 404, { message: "No encontrado" });
        });
    }
    _buildInfoPayload() {
        return {
            alias: this._alias,
            version: PROTOCOL_VERSION,
            deviceModel: null,
            deviceType: DeviceType.Desktop,
            fingerprint: this._fingerprint,
            download: false,
        };
    }
    _buildRegisterPayload() {
        return {
            ...this._buildInfoPayload(),
            port: this._httpPort,
            protocol: ProtocolType.Https,
        };
    }
    _rememberPeer(peer) {
        this._peers.set(peer.fingerprint, peer);
        this._callbacks.onStateChanged();
    }
    _prunePeers() {
        let changed = false;
        for (const [fingerprint, peer] of this._peers.entries()) {
            if (Date.now() - peer.lastSeenAt < PEER_STALE_MS)
                continue;
            this._peers.delete(fingerprint);
            changed = true;
        }
        if (changed)
            this._callbacks.onStateChanged();
    }
    _startDiscoverySocket() {
        if (this._multicastSocket !== null)
            return;
        const socket = Gio.Socket.new(Gio.SocketFamily.IPV4, Gio.SocketType.DATAGRAM, Gio.SocketProtocol.UDP);
        socket.set_blocking(false);
        socket.set_option(SOCKET_LEVEL_SOL, SOCKET_OPTION_REUSEADDR, 1);
        try {
            socket.set_option(SOCKET_LEVEL_SOL, SOCKET_OPTION_REUSEPORT, 1);
        }
        catch (error) {
            const message = error.message ?? String(error);
            if (DEBUG_LOGGING)
                console.warn(`LocalSend could not enable UDP port sharing: ${message}`);
        }
        const address = Gio.InetSocketAddress.new(Gio.InetAddress.new_from_string("0.0.0.0"), DEFAULT_PORT);
        if (!socket.bind(address, true))
            throw new Error(`Could not bind multicast socket on port ${DEFAULT_PORT}.`);
        const multicastGroup = Gio.InetAddress.new_from_string(DEFAULT_MULTICAST_GROUP);
        if (!socket.join_multicast_group(multicastGroup, false, null))
            throw new Error(`Could not join multicast group ${DEFAULT_MULTICAST_GROUP}.`);
        const source = socket.create_source(GLib.IOCondition.IN, null);
        source.set_callback(() => {
            this._readDiscoveryPackets();
            return GLib.SOURCE_CONTINUE;
        });
        this._multicastSourceId = source.attach(null);
        this._multicastSocket = socket;
    }
    _sendAnnouncement() {
        this._sendMulticastPacket({
            ...this._buildRegisterPayload(),
            announce: true,
            announcement: true,
        });
    }
    _readDiscoveryPackets() {
        if (this._multicastSocket === null)
            return;
        try {
            const [data, address] = this._multicastSocket.receive_bytes_from(4096, -1, null);
            if (address === null || data.get_size() === 0)
                return;
            const inet = address;
            const ip = inet.address.to_string();
            if (ip === null || ip.length === 0)
                return;
            const payload = decodeJson(data.get_data() ?? new Uint8Array());
            this._handleDiscoveryPacket(ip, payload);
        }
        catch (error) {
            const message = error.message ?? String(error);
            if (DEBUG_LOGGING)
                console.warn(`LocalSend discovery packet failed: ${message}`);
        }
    }
    _handleDiscoveryPacket(ip, payload) {
        const fingerprint = payload.fingerprint?.trim();
        const alias = payload.alias?.trim();
        if (fingerprint === undefined ||
            fingerprint.length === 0 ||
            alias === undefined ||
            alias.length === 0)
            return;
        if (fingerprint === this._fingerprint)
            return;
        const peer = {
            alias,
            version: payload.version?.trim() || PROTOCOL_VERSION,
            deviceModel: payload.deviceModel ?? null,
            deviceType: payload.deviceType ?? DeviceType.Desktop,
            fingerprint,
            port: payload.port || DEFAULT_PORT,
            protocol: payload.protocol ?? ProtocolType.Http,
            download: payload.download ?? false,
            ip,
            lastSeenAt: Date.now(),
        };
        this._rememberPeer(peer);
        if (payload.announce ?? payload.announcement) {
            void this._respondToAnnouncement(peer);
        }
    }
    async _respondToAnnouncement(peer) {
        try {
            await this._requestJson("POST", peer, "/api/localsend/v2/register", this._buildRegisterPayload());
        }
        catch (error) {
            const message = error.message ?? String(error);
            if (DEBUG_LOGGING)
                console.warn(`LocalSend register response failed for ${peer.alias}: ${message}`);
            this._sendUdpResponse();
        }
    }
    _sendUdpResponse() {
        this._sendMulticastPacket({
            ...this._buildRegisterPayload(),
            announce: false,
            announcement: false,
        });
    }
    _sendMulticastPacket(payload) {
        if (this._multicastSocket === null)
            return;
        try {
            this._multicastSocket.send_to(Gio.InetSocketAddress.new(Gio.InetAddress.new_from_string(DEFAULT_MULTICAST_GROUP), DEFAULT_PORT), encodeJson(payload).get_data() ?? new Uint8Array(), null);
        }
        catch (error) {
            const message = error.message ?? String(error);
            if (DEBUG_LOGGING)
                console.warn(`LocalSend multicast response failed: ${message}`);
        }
    }
    _handleRegister(message) {
        try {
            const request = decodeJson(this._requestBodyBytes(message));
            if (request.fingerprint === this._fingerprint) {
                this._respondJson(message, 412, { message: "Self-discovered" });
                return;
            }
            const { query } = parseRequestUrl(message);
            this._rememberPeer({
                ...request,
                ip: query.ip ?? this._remoteIp(message),
                lastSeenAt: Date.now(),
            });
            this._respondJson(message, 200, this._buildInfoPayload());
        }
        catch (error) {
            const messageText = error.message ?? String(error);
            this._respondJson(message, 400, { message: messageText });
        }
    }
    async _handlePrepareUpload(message) {
        try {
            const request = decodeJson(this._requestBodyBytes(message));
            const sender = request.info;
            if (sender.fingerprint === this._fingerprint) {
                this._respondJson(message, 412, { message: "Self-discovered" });
                return;
            }
            const files = Object.values(request.files);
            if (files.length === 0) {
                this._respondJson(message, 400, {
                    message: "La solicitud debe contener al menos un archivo",
                });
                return;
            }
            const peer = {
                alias: sender.alias,
                version: sender.version,
                deviceModel: sender.deviceModel ?? null,
                deviceType: sender.deviceType ?? DeviceType.Desktop,
                fingerprint: sender.fingerprint,
                port: sender.port,
                protocol: sender.protocol,
                download: sender.download,
                ip: this._remoteIp(message),
                lastSeenAt: Date.now(),
            };
            const [first] = files;
            if (files.length === 1 &&
                first.fileType === "text/plain" &&
                typeof first.preview === "string") {
                this._respondJson(message, 204, null);
                this._callbacks.onTextReceived(peer, first.preview);
                return;
            }
            if (this._incomingSession !== null) {
                this._respondJson(message, 409, {
                    message: "Bloqueado por otra sesión",
                });
                return;
            }
            const accepted = await this._callbacks.onIncomingTransfer({
                sender: peer,
                files,
                totalBytes: files.reduce((sum, file) => sum + file.size, 0),
            });
            if (!accepted) {
                this._respondJson(message, 403, {
                    message: REJECT_MESSAGE,
                });
                return;
            }
            if (!this._isConnected(message)) {
                this._callbacks.onNotification("LocalSend", `${peer.alias} canceló la transferencia.`);
                return;
            }
            const sessionId = GLib.uuid_string_random();
            const tokens = new Map();
            for (const file of files) {
                tokens.set(file.id, {
                    file,
                    token: GLib.uuid_string_random(),
                    path: null,
                    received: false,
                });
            }
            this._incomingSession = {
                sessionId,
                sender: peer,
                requestIp: peer.ip,
                destinationFolder: this._downloadFolder,
                files: tokens,
            };
            this._respondJson(message, 200, {
                sessionId,
                files: Object.fromEntries([...tokens.entries()].map(([fileId, entry]) => [fileId, entry.token])),
            });
        }
        catch (error) {
            const messageText = error.message ?? String(error);
            this._respondJson(message, 400, { message: messageText });
        }
        finally {
            message.unpause();
        }
    }
    _respondUpload(message) {
        try {
            const { query } = parseRequestUrl(message);
            const sessionId = query.sessionId ?? null;
            const fileId = query.fileId ?? null;
            const token = query.token ?? null;
            if (sessionId === null || fileId === null || token === null) {
                this._respondJson(message, 400, { message: "Faltan parámetros" });
                return;
            }
            if (this._incomingSession === null) {
                this._respondJson(message, 409, { message: "No hay ninguna sesión activa" });
                return;
            }
            if (this._incomingSession.sessionId !== sessionId) {
                this._respondJson(message, 403, { message: "ID de sesión no válido" });
                return;
            }
            const remoteIp = this._remoteIp(message);
            if (remoteIp !== this._incomingSession.requestIp) {
                this._respondJson(message, 403, {
                    message: `Invalid IP address: ${remoteIp}`,
                });
                return;
            }
            const fileEntry = this._incomingSession.files.get(fileId);
            if (fileEntry === undefined || fileEntry.token !== token) {
                this._respondJson(message, 403, { message: "Token no válido" });
                return;
            }
            const bytes = this._requestBodyBytes(message);
            const fileName = sanitizeFileName(fileEntry.file.fileName);
            const targetPath = this._makeUniquePath(this._incomingSession.destinationFolder, fileName);
            if (!GLib.file_set_contents(targetPath, bytes))
                throw new Error(`Failed to write ${targetPath}.`);
            fileEntry.path = targetPath;
            fileEntry.received = true;
            this._respondJson(message, 200, null);
            if ([...this._incomingSession.files.values()].every((entry) => entry.received)) {
                this._callbacks.onNotification("LocalSend", `Archivos guardados en ${this._incomingSession.destinationFolder}.`, Gio.File.new_for_path(this._incomingSession.destinationFolder).get_uri());
                this._incomingSession = null;
            }
        }
        catch (error) {
            const messageText = error.message ?? String(error);
            this._respondJson(message, 500, { message: messageText });
        }
    }
    _handleCancel(message) {
        const { query } = parseRequestUrl(message);
        const sessionId = query.sessionId ?? null;
        if (sessionId !== null &&
            this._incomingSession !== null &&
            this._incomingSession.sessionId === sessionId)
            this._incomingSession = null;
        this._respondJson(message, 200, null);
    }
    _isConnected(message) {
        const socket = message.get_socket();
        return (socket !== null &&
            socket.condition_check(GLib.IOCondition.IN | GLib.IOCondition.HUP | GLib.IOCondition.ERR) === 0);
    }
    _requestBodyBytes(message) {
        return message.get_request_body().flatten().get_data() ?? new Uint8Array();
    }
    _respondJson(message, statusCode, body) {
        message.set_status(statusCode, HTTP_STATUS_PHRASES[statusCode] ?? "");
        if (body === null) {
            message.set_response(null, Soup.MemoryUse.COPY, null);
            return;
        }
        message.set_response("application/json; charset=utf-8", Soup.MemoryUse.COPY, JSON.stringify(body));
    }
    async _sendOutgoingItems(peer, items) {
        const files = {};
        const buffers = new Map();
        for (const item of items) {
            const id = GLib.uuid_string_random();
            const bytes = item.bytes;
            buffers.set(id, bytes);
            files[id] = {
                id,
                fileName: sanitizeFileName(item.fileName),
                size: bytes.length,
                fileType: item.mimeType,
                preview: item.preview ?? null,
            };
        }
        let pin = null;
        let prepare;
        for (;;) {
            const pinQuery = pin === null ? "" : `?pin=${encodeURIComponent(pin)}`;
            prepare = await this._requestJson("POST", peer, `/api/localsend/v2/prepare-upload${pinQuery}`, {
                info: this._buildRegisterPayload(),
                files,
            });
            if (prepare.status !== 401)
                break;
            pin = await this._callbacks.requestPin(peer, pin !== null);
            if (pin === null)
                return;
        }
        if (prepare.status === 204) {
            this._callbacks.onNotification("LocalSend", `Enviado a ${peer.alias}.`);
            return;
        }
        if (prepare.status === 403) {
            this._callbacks.onNotification("LocalSend", REJECT_MESSAGE);
            return;
        }
        if (prepare.status === 429)
            throw new Error("Demasiados intentos de PIN incorrectos. Inténtalo más tarde.");
        if (prepare.status !== 200)
            throw new Error(`An error occurred: ${prepare.status} ${HTTP_STATUS_PHRASES[prepare.status]}.`);
        const response = decodeJson(prepare.body);
        const acceptedIds = Object.keys(response.files);
        if (acceptedIds.length === 0)
            throw new Error(REJECT_MESSAGE);
        await Promise.all(acceptedIds.map(async (fileId) => {
            const token = response.files[fileId];
            const file = files[fileId];
            const bytes = buffers.get(fileId);
            if (token === undefined || file === undefined || bytes === undefined)
                throw new Error(`Missing transfer data for file ${fileId}.`);
            await this._requestBinary("POST", peer, `/api/localsend/v2/upload?sessionId=${encodeURIComponent(response.sessionId)}&fileId=${encodeURIComponent(fileId)}&token=${encodeURIComponent(token)}`, bytes);
        }));
        this._callbacks.onNotification("LocalSend", `Se enviaron ${acceptedIds.length} archivo${acceptedIds.length === 1 ? "" : "s"} a ${peer.alias}.`);
    }
    async _requestJson(method, peer, path, payload) {
        const message = Soup.Message.new(method, this._buildUrl(peer, path));
        message.set_request_body_from_bytes("application/json", encodeJson(payload));
        if (peer.protocol === ProtocolType.Https) {
            message.connect("accept-certificate", () => true);
            message.set_tls_client_certificate(this._clientCert);
        }
        return this._sendAndRead(message);
    }
    async _requestBinary(method, peer, path, bytes) {
        const message = Soup.Message.new(method, this._buildUrl(peer, path));
        message.set_request_body_from_bytes("application/octet-stream", GLib.Bytes.new(bytes));
        if (peer.protocol === ProtocolType.Https) {
            message.connect("accept-certificate", () => true);
            message.set_tls_client_certificate(this._clientCert);
        }
        const response = await this._sendAndRead(message);
        if (response.status !== 200)
            throw new Error(`La subida mediante LocalSend falló con HTTP ${response.status}.`);
    }
    _buildUrl(peer, path) {
        const protocol = peer.protocol === ProtocolType.Https ? "https" : "http";
        return `${protocol}://${peer.ip}:${peer.port}${path}`;
    }
    async _sendAndRead(message) {
        const bytes = await this._session.send_and_read_async(message, 0, this._cancellable);
        return {
            status: message.get_status(),
            body: bytes.get_data() ?? new Uint8Array(),
        };
    }
    _makeUniquePath(folder, fileName) {
        GLib.mkdir_with_parents(folder, 0o755);
        const base = sanitizeFileName(fileName);
        const extensionIndex = base.lastIndexOf(".");
        const name = extensionIndex > 0 ? base.slice(0, extensionIndex) : base;
        const extension = extensionIndex > 0 ? base.slice(extensionIndex) : "";
        let candidate = GLib.build_filenamev([folder, base]);
        for (let index = 1; Gio.File.new_for_path(candidate).query_exists(null); index++) {
            candidate = GLib.build_filenamev([
                folder,
                `${name} (${index})${extension}`,
            ]);
        }
        return candidate;
    }
    _remoteIp(message) {
        const remoteAddress = message.get_remote_address();
        if (remoteAddress instanceof Gio.InetSocketAddress) {
            const ip = remoteAddress.address.to_string();
            if (ip !== null)
                return ip;
        }
        return this._incomingSession?.requestIp ?? "127.0.0.1";
    }
    _listenOnHttpPort(preferredPort) {
        const tryListen = (port) => {
            if (!this._server.listen_all(port, Soup.ServerListenOptions.HTTPS))
                throw new Error(`Failed to listen on port ${port}.`);
            const uris = this._server.get_uris();
            for (const uri of uris) {
                const portNumber = uri.get_port();
                if (portNumber > 0)
                    return portNumber;
            }
            throw new Error("El servidor HTTP de LocalSend no informó de un puerto de escucha.");
        };
        try {
            return tryListen(preferredPort);
        }
        catch (error) {
            if (!(error instanceof GLib.Error) ||
                error.code !== Gio.IOErrorEnum.ADDRESS_IN_USE)
                throw error;
            const fallbackPort = tryListen(0);
            this._callbacks.onNotification("LocalSend", `El puerto ${preferredPort} ya está en uso. LocalSend utilizará el puerto ${fallbackPort}.`);
            return fallbackPort;
        }
    }
}
