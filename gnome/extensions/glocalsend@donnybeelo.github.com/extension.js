import Clutter from "gi://Clutter";
import Gio from "gi://Gio";
import Graphene from "gi://Graphene";
import GLib from "gi://GLib";
import St from "gi://St";
import Shell from "gi://Shell";
import GObject from "gi://GObject";
import * as Main from "resource:///org/gnome/shell/ui/main.js";
import * as QuickSettings from "resource:///org/gnome/shell/ui/quickSettings.js";
import * as PopupMenu from "resource:///org/gnome/shell/ui/popupMenu.js";
import * as ModalDialog from "resource:///org/gnome/shell/ui/modalDialog.js";
import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";
import * as CertificateDialog from "./certificateDialog.js";
import { DeviceType, formatBytes, KEY_AUTO_ACCEPT } from "./common.js";
import { LocalSendService, } from "./localsend.js";
import { SharingBridge } from "./sharingBridge.js";
Gio._promisify(Gio.DBusProxy.prototype, "call", "call_finish");
const DEFAULT_PEER_ICON = "network-workgroup-symbolic";
const SPIN_MS = 10000;
const STOP_STEP = 45;
const PEER_DEVICE_ICONS = {
    [DeviceType.Mobile]: "smartphone-symbolic",
    [DeviceType.Desktop]: "computer-symbolic",
    [DeviceType.Web]: "globe-symbolic",
    [DeviceType.Headless]: "terminal-symbolic",
    [DeviceType.Server]: "network-server-symbolic",
};
function getPeerIconName(peer) {
    if (peer.deviceType === null || peer.deviceType === undefined)
        return DEFAULT_PEER_ICON;
    return PEER_DEVICE_ICONS[peer.deviceType] ?? DEFAULT_PEER_ICON;
}
const FileChooserXml = `
<node>
  <interface name="org.freedesktop.portal.FileChooser">
    <method name="OpenFile">
      <arg type="s" name="parent_window" direction="in"/>
      <arg type="s" name="title" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="handle" direction="out"/>
    </method>
  </interface>
  <interface name="org.freedesktop.portal.Request">
    <signal name="Response">
      <arg type="u" name="response"/>
      <arg type="a{sv}" name="results"/>
    </signal>
  </interface>
</node>`;
const LocalSendToggle = GObject.registerClass({ GTypeName: "GLocalSend_LocalSendToggle" }, class LocalSendToggle extends QuickSettings.QuickMenuToggle {
    constructor(iconPath) {
        super({
            title: "LocalSend",
            subtitle: "Iniciando",
            gicon: Gio.icon_new_for_string(iconPath),
            menuEnabled: true,
        });
    }
});
const LocalSendIndicator = GObject.registerClass({ GTypeName: "GLocalSend_LocalSendIndicator" }, class LocalSendIndicator extends QuickSettings.SystemIndicator {
    _indicator;
    toggle;
    _spin;
    _pauseId = 0;
    constructor(iconPath) {
        super();
        this._indicator = this._addIndicator();
        this._indicator.gicon = Gio.icon_new_for_string(iconPath);
        this._indicator.visible = false;
        this.toggle = new LocalSendToggle(iconPath);
        this.quickSettingsItems.push(this.toggle);
        this._spin = new Clutter.PropertyTransition({
            property_name: "rotation-angle-z",
            duration: SPIN_MS,
            progress_mode: Clutter.AnimationMode.LINEAR,
            repeat_count: -1,
        });
        this._spin.set_from(0);
        this._spin.set_to(360);
        this._indicator.pivot_point = new Graphene.Point({ x: 0.5, y: 0.5 });
        this._indicator.add_transition("rotation-angle-z", this._spin);
        this._spin.pause();
        const inner = this.toggle._box.get_first_child();
        for (const a of [inner._icon, this.toggle.menu._headerIcon]) {
            a.pivot_point = this._indicator.pivot_point;
            this._indicator.bind_property("rotation-angle-z", a, "rotation-angle-z", GObject.BindingFlags.SYNC_CREATE);
        }
    }
    setSpinning(on) {
        if (this._pauseId)
            GLib.source_remove(this._pauseId);
        this._pauseId = 0;
        if (on) {
            if (St.Settings.get().enable_animations)
                this._spin.start();
            return;
        }
        const angle = this._indicator.rotation_angle_z;
        const target = Math.min(Math.ceil(angle / STOP_STEP) * STOP_STEP, 360);
        const ms = Math.round(((target - angle) / 360) * SPIN_MS);
        this._pauseId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
            this._spin.pause();
            this._pauseId = 0;
            return GLib.SOURCE_REMOVE;
        });
    }
    destroy() {
        if (this._pauseId)
            GLib.source_remove(this._pauseId);
        this.quickSettingsItems?.forEach((i) => {
            i.destroy();
        });
        this._indicator.destroy();
        this._indicator = null;
        super.destroy();
    }
});
const TextPromptDialog = GObject.registerClass({ GTypeName: "GLocalSend_TextPromptDialog" }, class TextPromptDialog extends ModalDialog.ModalDialog {
    _titleLabel = null;
    _descriptionLabel = null;
    _errorLabel = null;
    _entry = null;
    _resolve = null;
    _content = null;
    _activateId = 0;
    constructor() {
        super({
            shellReactive: true,
            actionMode: Shell.ActionMode.ALL,
            shouldFadeIn: true,
            shouldFadeOut: true,
            destroyOnClose: false,
        });
        this._content = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
            y_expand: true,
            style_class: "prompt-dialog-content",
        });
        this._titleLabel = new St.Label({
            style_class: "prompt-dialog-title",
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._descriptionLabel = new St.Label({
            style_class: "prompt-dialog-description",
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._entry = new St.Entry({
            hint_text: "Escribe el texto que quieres enviar",
            x_expand: true,
        });
        this._errorLabel = new St.Label({
            style_class: "prompt-dialog-error",
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._content.add_child(this._titleLabel);
        this._content.add_child(this._descriptionLabel);
        this._content.add_child(this._entry);
        this._content.add_child(this._errorLabel);
        this.contentLayout.add_child(this._content);
        this.setInitialKeyFocus(this._entry);
        this._activateId = this._entry.clutter_text.connect("activate", () => {
            this._submit();
        });
        this.setButtons([
            {
                label: "Cancelar",
                action: () => {
                    this._resolvePrompt(null);
                },
            },
            {
                label: "Enviar",
                default: true,
                action: () => {
                    this._submit();
                },
            },
        ]);
    }
    prompt(title, description, initialText = "") {
        if (this._resolve !== null)
            this._resolvePrompt(null);
        this._titleLabel.text = title;
        this._descriptionLabel.text = description;
        this._errorLabel.text = "";
        this._entry.text = initialText;
        return new Promise((resolve) => {
            this._resolve = resolve;
            this.open();
            this.setInitialKeyFocus(this._entry);
            this._entry.grab_key_focus();
        });
    }
    destroy() {
        this._resolvePrompt(null);
        this._titleLabel.destroy();
        this._titleLabel = null;
        this._descriptionLabel.destroy();
        this._descriptionLabel = null;
        this._errorLabel.destroy();
        this._errorLabel = null;
        this._entry.clutter_text.disconnect(this._activateId);
        this._entry.destroy();
        this._entry = null;
        this._content.destroy();
        this._content = null;
        super.destroy();
    }
    _submit() {
        const text = this._entry.text;
        if (text.trim().length === 0) {
            this._errorLabel.text = "Escribe un texto antes de enviarlo.";
            this._entry.grab_key_focus();
            return;
        }
        this._resolvePrompt(text);
    }
    _resolvePrompt(value) {
        const resolve = this._resolve;
        this._resolve = null;
        if (resolve !== null)
            resolve(value);
        this.close();
    }
});
const ReceivedTextDialog = GObject.registerClass({ GTypeName: "GLocalSend_ReceivedTextDialog" }, class ReceivedTextDialog extends ModalDialog.ModalDialog {
    _senderLabel = null;
    _textLabel = null;
    constructor() {
        super({
            shellReactive: true,
            actionMode: Shell.ActionMode.ALL,
            shouldFadeIn: true,
            shouldFadeOut: true,
            destroyOnClose: true,
        });
        this._senderLabel = new St.Label({
            style_class: "prompt-dialog-title",
            x_align: Clutter.ActorAlign.START,
        });
        this._textLabel = new St.Label({
            style_class: "prompt-dialog-description",
            x_align: Clutter.ActorAlign.START,
        });
        this._textLabel.clutter_text.line_wrap = true;
        this._textLabel.clutter_text.selectable = true;
        this.contentLayout.add_child(this._senderLabel);
        this.contentLayout.add_child(this._textLabel);
    }
    present(senderAlias, text) {
        const isUrl = /^https?:\/\//i.test(text.trim());
        this._senderLabel.text = `${senderAlias} te ha enviado ${isUrl ? "un enlace" : "un mensaje"}:`;
        this._textLabel.text = text;
        this.setButtons([
            { label: "Cerrar", action: () => this.close() },
            {
                label: "Copiar",
                default: !isUrl,
                action: () => {
                    St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text);
                    this.close();
                },
            },
            ...(isUrl
                ? [
                    {
                        label: "Abrir",
                        default: true,
                        action: () => {
                            void Gio.AppInfo.launch_default_for_uri(text.trim(), null);
                            this.close();
                        },
                    },
                ]
                : []),
        ]);
        this.open();
    }
    destroy() {
        this._senderLabel?.destroy();
        this._senderLabel = null;
        this._textLabel?.destroy();
        this._textLabel = null;
        super.destroy();
    }
});
const IncomingTransferDialog = GObject.registerClass({ GTypeName: "GLocalSend_IncomingTransferDialog" }, class IncomingTransferDialog extends ModalDialog.ModalDialog {
    _summaryLabel = null;
    _filesLabel = null;
    _content = null;
    _resolve = null;
    constructor() {
        super({
            shellReactive: true,
            actionMode: Shell.ActionMode.ALL,
            shouldFadeIn: true,
            shouldFadeOut: true,
            destroyOnClose: false,
        });
        this._content = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
            y_expand: true,
            style_class: "prompt-dialog-content",
        });
        this._summaryLabel = new St.Label({
            style_class: "prompt-dialog-title",
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._filesLabel = new St.Label({
            style_class: "prompt-dialog-description",
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._content.add_child(this._summaryLabel);
        this._content.add_child(this._filesLabel);
        this.contentLayout.add_child(this._content);
        this.setButtons([
            {
                label: "Rechazar",
                action: () => {
                    this._resolvePrompt(false);
                },
            },
            {
                label: "Aceptar",
                default: true,
                action: () => {
                    this._resolvePrompt(true);
                },
            },
        ]);
    }
    prompt(sender, request) {
        if (this._resolve !== null)
            this._resolvePrompt(false);
        this._summaryLabel.text = `${sender.alias} quiere enviarte ${request.files.length} archivo${request.files.length === 1 ? "" : "s"}.`;
        this._filesLabel.text = request.files
            .map((file) => `${file.fileName} (${formatBytes(file.size)})`)
            .join("\n");
        return new Promise((resolve) => {
            this._resolve = resolve;
            this.open();
        });
    }
    destroy() {
        this._resolvePrompt(false);
        this._summaryLabel.destroy();
        this._summaryLabel = null;
        this._filesLabel.destroy();
        this._filesLabel = null;
        this._content?.destroy();
        this._content = null;
        super.destroy();
    }
    _resolvePrompt(value) {
        const resolve = this._resolve;
        this._resolve = null;
        if (resolve !== null)
            resolve(value);
        this.close();
    }
});
export default class LocalSendCompanionExtension extends Extension {
    _settings;
    _startOnLoginSourceId = 0;
    _sharingBridge = null;
    _indicator = null;
    _indicatorClickedSignalId = null;
    _service = null;
    _textPromptDialog = null;
    _incomingDialog = null;
    _knownPeers = new Map();
    get _iconPath() {
        return `file://${this.path}/icon-symbolic.svg`;
    }
    enable() {
        this._settings = this.getSettings();
        this._service = new LocalSendService(this._settings, {
            onStateChanged: () => {
                this._syncIndicator();
            },
            onNotification: (summary, body, actionUri) => {
                const notification = Main.notify(summary, body);
                if (actionUri !== undefined) {
                    void Gio.AppInfo.launch_default_for_uri(actionUri, null);
                }
            },
            onIncomingTransfer: async (request) => {
                if (this._settings.get_boolean(KEY_AUTO_ACCEPT))
                    return true;
                const dialog = this._ensureIncomingDialog();
                return await dialog.prompt(request.sender, request);
            },
            onTextReceived: (sender, text) => {
                new ReceivedTextDialog().present(sender.alias, text);
            },
            onCertificateMissing: () => CertificateDialog.show(),
            requestPin: (peer, retry) => this._ensureTextPromptDialog().prompt(`PIN for ${peer.alias}`, retry
                ? "PIN incorrecto. Inténtalo de nuevo."
                : `${peer.alias} necesita un PIN antes de aceptar archivos.`),
        });
        this._sharingBridge = new SharingBridge(this._service, this._settings);
        this._indicator = new LocalSendIndicator(this._iconPath);
        this._indicatorClickedSignalId = this._indicator.toggle.connect("clicked", () => {
            void this._runUserAction("Toggle LocalSend", async () => {
                this._service?.toggleEnabled();
            });
        });
        // statusArea.quickSettings: private API, required for Quick Settings integration; GNOME 51 integration point
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        // Start LocalSend automatically once the GNOME session is ready.
        if (this._settings.get_boolean("start-on-login")) {
            this._startOnLoginSourceId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 3, () => {
                this._startOnLoginSourceId = 0;
                if (this._service !== null && !this._service.enabled) {
                    try {
                        this._service.toggleEnabled();
                        this._syncIndicator();
                    } catch (error) {
                        console.error(`GLocalSend: fallo al iniciar automáticamente: ${error}`);
                    }
                }

                return GLib.SOURCE_REMOVE;
            });
        }

        // Auto-disable timing is configurable in preferences.
        this._service?.stop();
        this._syncIndicator();
    }
    disable() {
        if (this._startOnLoginSourceId) {
            GLib.source_remove(this._startOnLoginSourceId);
            this._startOnLoginSourceId = 0;
        }
        this._sharingBridge?.destroy();
        this._sharingBridge = null;
        this._service?.stop();
        this._service = null;
        this._knownPeers.clear();
        this._textPromptDialog?.destroy();
        this._textPromptDialog = null;
        this._incomingDialog?.destroy();
        this._incomingDialog = null;
        if (this._indicator !== null && this._indicatorClickedSignalId !== null) {
            this._indicator.toggle.disconnect(this._indicatorClickedSignalId);
            this._indicatorClickedSignalId = null;
        }
        if (this._indicator !== null) {
            this._indicator.destroy();
            this._indicator = null;
        }
        this._settings = null;
    }
    _buildPeerItem(peer) {
        const favorites = new Set(this._settings.get_strv("favorite-fingerprints"));
        const isFavorite = favorites.has(peer.fingerprint);

        const peerItem = new PopupMenu.PopupSubMenuMenuItem(
            isFavorite ? `★ ${peer.alias}` : peer.alias,
            true
        );
        const peerIcon = peerItem.icon;
        if (peerIcon !== undefined) {
            peerIcon.gicon = Gio.icon_new_for_string(getPeerIconName(peer));
        }
        peerItem.menu.addAction("Enviar archivos", () => {
            void this._sendFilesToPeer(peer);
        }, Gio.icon_new_for_string("document-send-symbolic"));
        peerItem.menu.addAction("Enviar portapapeles", () => {
            void this._sendClipboardToPeer(peer);
        }, Gio.icon_new_for_string("edit-paste-symbolic"));
        peerItem.menu.addAction("Escribir texto", () => {
            void this._promptAndSendText(peer);
        }, Gio.icon_new_for_string("insert-text-symbolic"));

        peerItem.menu.addAction(
            isFavorite ? "Quitar de favoritos" : "Añadir a favoritos",
            () => {
                const current = new Set(
                    this._settings.get_strv("favorite-fingerprints")
                );

                if (current.has(peer.fingerprint))
                    current.delete(peer.fingerprint);
                else
                    current.add(peer.fingerprint);

                this._settings.set_strv(
                    "favorite-fingerprints",
                    [...current]
                );

                this._syncIndicator();
            },
            Gio.icon_new_for_string(
                isFavorite ? "starred-symbolic" : "non-starred-symbolic"
            )
        );

        return peerItem;
    }
    _syncIndicator() {
        if (this._indicator === null || this._service === null)
            return;
        const enabled = this._service.enabled;
        this._indicator.setSpinning(enabled);
        const favorites = new Set(
            this._settings.get_strv("favorite-fingerprints")
        );

        const peers = [...this._service.peers].sort((left, right) => {
            const leftFavorite = favorites.has(left.fingerprint);
            const rightFavorite = favorites.has(right.fingerprint);

            if (leftFavorite !== rightFavorite)
                return leftFavorite ? -1 : 1;

            return left.alias.localeCompare(right.alias);
        });

        const alias = this._settings.get_string("alias");
        const subtitle = enabled ? alias : null;
        const subheader = !enabled
            ? "Inactivo"
            : peers.length === 0
                ? `${alias} - Buscando dispositivos cercanos`
                : `${alias} - ${peers.length} dispositivo${peers.length === 1 ? "" : "s"} cercano${peers.length === 1 ? "" : "s"}`;
        this._indicator.toggle.checked = enabled;
        this._indicator._indicator.visible = enabled;
        this._indicator.visible = enabled;
        this._indicator.toggle.checked = enabled;
        this._indicator.toggle.subtitle = subtitle;
        this._indicator.toggle.menu.setHeader(Gio.icon_new_for_string(this._iconPath), "LocalSend", subheader);
        const previousPeers = this._knownPeers;
        const currentFingerprints = new Set(peers.map((peer) => peer.fingerprint));
        const leavingPeers = [...previousPeers.values()].filter((peer) => !currentFingerprints.has(peer.fingerprint));
        this._indicator.toggle.menu.removeAll();
        if (enabled) {
            for (const peer of peers) {
                const isNewPeer = !previousPeers.has(peer.fingerprint);
                const peerItem = this._buildPeerItem(peer);
                if (isNewPeer) {
                    peerItem.opacity = 0;
                    peerItem.ease({ opacity: 255, duration: 200 });
                }
                this._indicator.toggle.menu.addMenuItem(peerItem);
            }
            for (const peer of leavingPeers) {
                const peerItem = this._buildPeerItem(peer);
                this._indicator.toggle.menu.addMenuItem(peerItem);
                peerItem.ease({
                    opacity: 0,
                    duration: 200,
                    onStopped: () => {
                        if (peerItem.get_parent() !== null)
                            peerItem.destroy();
                    },
                });
            }
        }
        this._knownPeers = new Map(peers.map((peer) => [peer.fingerprint, peer]));
        this._indicator.toggle.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        if (enabled) {
            const refreshItem = this._indicator.toggle.menu.addAction("Actualizar dispositivos cercanos", () => { }, Gio.icon_new_for_string("view-refresh-symbolic"));
            refreshItem.activate = () => {
                const icon = refreshItem._icon;
                icon.pivot_point = new Graphene.Point({ x: 0.5, y: 0.5 });
                icon.ease({ rotationAngleZ: 0, duration: 0 });
                icon.ease({
                    rotation_angle_z: 360,
                    duration: 300,
                    mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
                    onComplete: () => this._service?.refreshPeers(),
                });
            };
        }
        this._indicator.toggle.menu.addAction("Ajustes de LocalSend", () => {
            this.openPreferences();
        });
    }
    async _sendFilesToPeer(peer) {
        await this._runUserAction(`Enviar archivos a ${peer.alias}`, async () => {
            if (this._service === null)
                throw new Error("El servicio de LocalSend no está disponible.");
            const fileChooserInterfaceInfo = Gio.DBusNodeInfo.new_for_xml(FileChooserXml).lookup_interface("org.freedesktop.portal.FileChooser");
            if (fileChooserInterfaceInfo === null) {
                throw new Error("No se pudo cargar la interfaz D-Bus del selector de archivos.");
            }
            const proxy = new Gio.DBusProxy({
                g_connection: Gio.DBus.session,
                g_name: "org.freedesktop.portal.Desktop",
                g_object_path: "/org/freedesktop/portal/desktop",
                g_interface_name: "org.freedesktop.portal.FileChooser",
                g_interface_info: fileChooserInterfaceInfo,
            });
            const options = {
                modal: GLib.Variant.new_boolean(false),
                multiple: GLib.Variant.new_boolean(true),
                accept_label: GLib.Variant.new_string("Seleccionar"),
            };
            const callResult = await proxy.call("OpenFile", GLib.Variant.new("(ssa{sv})", ["", "Seleccionar archivos", options]), Gio.DBusCallFlags.NONE, -1, null);
            const [handle] = callResult.recursiveUnpack();
            const selectedUris = await new Promise((resolve, reject) => {
                let subscriptionId = 0;
                subscriptionId = Gio.DBus.session.signal_subscribe("org.freedesktop.portal.Desktop", "org.freedesktop.portal.Request", "Response", handle, null, Gio.DBusSignalFlags.NONE, (_connection, _senderName, _objectPath, _interfaceName, _signalName, parameters) => {
                    try {
                        Gio.DBus.session.signal_unsubscribe(subscriptionId);
                        const [response, results] = parameters.recursiveUnpack();
                        if (response !== 0) {
                            resolve([]);
                            return;
                        }
                        const urisValue = results.uris;
                        if (urisValue instanceof GLib.Variant) {
                            const unpacked = urisValue.recursiveUnpack();
                            resolve(Array.isArray(unpacked)
                                ? unpacked.filter((uri) => typeof uri === "string")
                                : []);
                            return;
                        }
                        resolve(Array.isArray(urisValue)
                            ? urisValue.filter((uri) => typeof uri === "string")
                            : []);
                    }
                    catch (error) {
                        reject(error);
                    }
                });
            });
            if (selectedUris.length === 0)
                return;
            const filePaths = selectedUris
                .map((uri) => Gio.File.new_for_uri(uri).get_path())
                .filter((path) => path !== null && path.length > 0);
            if (filePaths.length === 0)
                throw new Error("Solo se pueden enviar archivos locales.");
            await this._service.sendFilesToPeer(peer, filePaths);
        });
    }
    async _sendClipboardToPeer(peer) {
        await this._runUserAction(`Enviar portapapeles a ${peer.alias}`, async () => {
            const clipboard = St.Clipboard.get_default();
            const type = St.ClipboardType.CLIPBOARD;
            // clipboard is read only here, on explicit user menu action — never automatic
            const mimes = clipboard.get_mimetypes(type);
            const read = (mime) => new Promise((resolve) => {
                clipboard.get_content(type, mime, (_c, bytes) => {
                    resolve((bytes instanceof Uint8Array ? bytes : bytes?.get_data()) ??
                        new Uint8Array());
                });
            });
            if (mimes.includes("text/uri-list")) {
                const paths = new TextDecoder()
                    .decode(await read("text/uri-list"))
                    .split(/\r?\n/)
                    .filter((l) => l && !l.startsWith("#"))
                    .map((uri) => Gio.File.new_for_uri(uri).get_path())
                    .filter((p) => p !== null);
                if (paths.length > 0) {
                    await this._service?.sendFilesToPeer(peer, paths);
                    return;
                }
            }
            const image = mimes.includes("image/png")
                ? "image/png"
                : mimes.find((m) => m.startsWith("image/"));
            if (image) {
                await this._service?.sendClipboardImageToPeer(peer, await read(image), image);
                return;
            }
            const text = await new Promise((resolve) => {
                clipboard.get_text(type, (_c, value) => resolve(value ?? ""));
            });
            await this._service?.sendClipboardTextToPeer(peer, text);
        });
    }
    async _promptAndSendText(peer) {
        await this._runUserAction(`Enviar texto a ${peer.alias}`, async () => {
            const dialog = this._ensureTextPromptDialog();
            const text = await dialog.prompt(`Enviar texto a ${peer.alias}`, "Escribe o pega el texto que quieres enviar con LocalSend.");
            if (text === null)
                return;
            await this._service?.sendTypedTextToPeer(peer, text);
        });
    }
    _ensureTextPromptDialog() {
        if (this._textPromptDialog === null)
            this._textPromptDialog = new TextPromptDialog();
        return this._textPromptDialog;
    }
    _ensureIncomingDialog() {
        if (this._incomingDialog === null)
            this._incomingDialog = new IncomingTransferDialog();
        return this._incomingDialog;
    }
    async _runUserAction(title, action) {
        try {
            await action();
        }
        catch (error) {
            if (this._service === null)
                return;
            const message = error.message ?? String(error);
            Main.notifyError(title, message);
        }
    }
}
