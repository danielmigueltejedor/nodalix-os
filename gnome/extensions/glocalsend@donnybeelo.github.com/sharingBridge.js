import Gio from 'gi://Gio';
import {CERT_PATH, KEY_PATH} from './common.js';

const XML = `<node><interface name="com.nodalix.LocalSend1">
<method name="GetStatus"><arg name="status" type="s" direction="out"/></method>
<method name="Refresh"/>
<method name="SetEnabled"><arg type="b" direction="in"/><arg type="b" direction="out"/></method>
<signal name="Changed"/>
</interface></node>`;

// The extension already owns discovery. Export small cached metadata only;
// Nautilus starts a detached streaming worker instead of loading file data
// into the compositor or starting a second LocalSend receiver.
export class SharingBridge {
    constructor(service, settings) {
        this._service = service;
        this._settings = settings;
        this._object = Gio.DBusExportedObject.wrapJSObject(XML, this);
        this._object.export(Gio.DBus.session, '/com/nodalix/LocalSend');
        this._settingsId = settings.connect('changed', (_settings, key) => {
            this._service.applySettings(key);
            this.changed();
        });
        this._owner = Gio.bus_own_name_on_connection(Gio.DBus.session,
            'com.nodalix.LocalSend', Gio.BusNameOwnerFlags.NONE, null, null);
    }

    GetStatus() {
        const favorites = new Set(this._settings.get_strv('favorite-fingerprints'));
        const devices = this._service.peers.map(peer => ({...peer, favorite: favorites.has(peer.fingerprint)}));
        devices.sort((a, b) => Number(b.favorite) - Number(a.favorite) || a.alias.localeCompare(b.alias));
        return JSON.stringify({enabled: this._service.enabled, devices,
            identity: {alias: this._service.alias, fingerprint: this._service.fingerprint,
                port: this._service.httpPort, protocol: 'https', version: '2.1', deviceType: 'desktop', deviceModel: 'Nodalix'},
            certificate: CERT_PATH, key: KEY_PATH});
    }

    Refresh() {
        this._service.refreshPeers();
    }

    SetEnabled(enabled) {
        if (Boolean(enabled) !== this._service.enabled)
            this._service.toggleEnabled();
        this.changed();
        return this._service.enabled;
    }

    changed() {
        this._object?.emit_signal('Changed', null);
    }

    destroy() {
        if (this._settingsId)
            this._settings.disconnect(this._settingsId);
        this._settingsId = 0;
        if (this._owner)
            Gio.bus_unown_name(this._owner);
        this._owner = 0;
        this._object?.unexport();
        this._object = null;
        this._service = null;
        this._settings = null;
    }
}
