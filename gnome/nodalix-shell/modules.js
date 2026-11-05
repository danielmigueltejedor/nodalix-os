import Gio from 'gi://Gio';
import St from 'gi://St';

import {ExtensionState, ExtensionType, loadExtensionMetadata} from '../misc/extensionUtils.js';

const manifest = Gio.File.new_for_uri(import.meta.url).get_parent().get_child('modules.json');
const [, bytes] = manifest.load_contents(null);
const {bundled, desktopPackages} = JSON.parse(new TextDecoder().decode(bytes));
const bundledIds = new Set(bundled);
const packagedIds = new Set(desktopPackages);
export const PHONE_LINK_ID = 'nodalix-connect@getnodalia.com';
const records = new Map();

function moduleDirectory(uuid) {
    const share = Gio.File.new_for_path(global.datadir);
    if (bundledIds.has(uuid))
        return share.get_child('nodalix-modules').get_child(uuid);

    if (!packagedIds.has(uuid))
        return null;
    // Read the packaged engine only. The old package location works during a
    // partial migration and never resolves to a user-installed extension.
    for (const directory of ['nodalix-runtime', 'extensions']) {
        const module = share.get_child(directory).get_child(uuid);
        if (module.get_child('extension.js').query_exists(null))
            return module;
    }
    return null;
}

export function managesModule(uuid) {
    return uuid === PHONE_LINK_ID || moduleDirectory(uuid) !== null;
}

// Existing Shell integrations can discover a built-in peer by UUID. Discovery,
// enablement, and updates of third-party extensions remain separate.
export function lookupModule(uuid) {
    return records.get(uuid);
}

export class DesktopModules {
    constructor(sessionMode, extensionManager) {
        this.sessionMode = sessionMode;
        this.extensionManager = extensionManager;
        this.settingsSignals = ['enabled-extensions', 'disabled-extensions', 'disable-user-extensions']
            .map(key => global.settings.connect(`changed::${key}`, () => this.schedule()));
        this.sessionSignal = sessionMode.connect('updated', () => this.schedule());
        this.colorSignal = St.Settings.get().connect('notify::color-scheme', () => this.reloadStyles());
        this.schedule();
    }

    selectedModules() {
        const ids = this.sessionMode.enabledExtensions ?? [];
        const requested = global.settings.get_boolean('disable-user-extensions')
            ? ids : [...ids, ...global.settings.get_strv('enabled-extensions')];
        const disabled = new Set(global.settings.get_strv('disabled-extensions'));
        return [...new Set(requested)].filter(uuid => uuid !== PHONE_LINK_ID && managesModule(uuid) && !disabled.has(uuid));
    }

    supportsSession(record) {
        const modes = record.metadata['session-modes'] ?? ['user'];
        return modes.includes(this.sessionMode.currentMode) || modes.includes(this.sessionMode.parentMode);
    }

    schedule() {
        this.requested = true;
        if (this.running || this.stopped)
            return;
        this.running = true;
        this.reconcile().finally(() => {
            this.running = false;
            if (this.requested && !this.stopped)
                this.schedule();
        }).catch(error => logError(error, 'Nodalix Shell module reconciliation failed'));
    }

    async reconcile() {
        this.requested = false;
        const desired = this.selectedModules();
        for (const [uuid, record] of [...records].reverse()) {
            if (!desired.includes(uuid) || !this.supportsSession(record))
                this.disable(record);
        }
        for (const uuid of desired) {
            if (this.stopped || this.requested)
                return;
            if (!records.has(uuid))
                await this.enable(uuid);
        }
    }

    async enable(uuid) {
        let record;
        try {
            const dir = moduleDirectory(uuid);
            if (!dir)
                return;
            const metadata = loadExtensionMetadata(uuid, dir);
            record = {
                uuid, dir, metadata, path: dir.get_path(), type: ExtensionType.SYSTEM,
                state: ExtensionState.ACTIVATING, sessionModes: metadata['session-modes'] ?? ['user'],
                error: '', enabled: true, hasUpdate: false, canChange: false,
                hasPrefs: dir.get_child('prefs.js').query_exists(null),
            };
            if (!this.supportsSession(record))
                return;
            const {default: Component} = await import(dir.get_child('extension.js').get_uri());
            if (this.stopped || this.requested)
                return;
            record.stateObj = new Component({...metadata, dir, path: record.path});
            records.set(uuid, record);
            // Apply the same St stylesheet selection as the stock desktop.
            this.extensionManager._loadExtensionStylesheet(record);
            await record.stateObj.enable();
            if (this.stopped || !records.has(uuid))
                return;
            record.state = ExtensionState.ACTIVE;
            this.extensionManager.emit('extension-state-changed', record);
            console.info(`Nodalix Shell module active: ${uuid}`);
        } catch (error) {
            if (records.has(uuid))
                this.disable(record);
            logError(error, `Nodalix Shell module failed: ${uuid}`);
        }
    }

    disable(record) {
        try {
            if (this.stopped && typeof record.stateObj.shutdown === 'function')
                record.stateObj.shutdown();
            else
                record.stateObj.disable();
        } catch (error) {
            logError(error, `Nodalix Shell module cleanup failed: ${record.uuid}`);
        } finally {
            this.extensionManager._unloadExtensionStylesheet(record);
            record.enabled = false;
            record.state = ExtensionState.INACTIVE;
            this.extensionManager.emit('extension-state-changed', record);
            records.delete(record.uuid);
        }
    }

    reloadStyles() {
        for (const record of records.values()) {
            if (!/-(dark|light)\.css$/.test(record.stylesheet?.get_path() ?? ''))
                continue;
            this.extensionManager._unloadExtensionStylesheet(record);
            this.extensionManager._loadExtensionStylesheet(record);
        }
    }

    destroy() {
        this.stopped = true;
        for (const id of this.settingsSignals)
            global.settings.disconnect(id);
        this.sessionMode.disconnect(this.sessionSignal);
        St.Settings.get().disconnect(this.colorSignal);
        for (const record of [...records.values()].reverse())
            this.disable(record);
    }
}
