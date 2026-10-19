import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class StartupCursor extends Extension {
    enable() {
        this._seen = new Map();
        this._source = 0;
        this._notification = Shell.WindowTracker.get_default();
        this._changed = this._notification.connect('startup-sequence-changed', () => this._schedule());
        this._schedule();
    }

    _schedule() {
        if (this._source)
            return;
        this._source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
            const pending = this._check();
            if (!pending)
                this._source = 0;
            return pending ? GLib.SOURCE_CONTINUE : GLib.SOURCE_REMOVE;
        });
    }

    _windows(sequence) {
        const id = sequence.get_application_id();
        if (!id)
            return [];
        // Mutter may use the desktop ID or its full filename.
        const desktopId = id.split('/').pop();
        const app = Shell.AppSystem.get_default().lookup_app(desktopId);
        return app?.get_windows() ?? [];
    }

    _check() {
        const sequences = this._notification.get_startup_sequences() ?? [];
        const active = new Set();
        const now = GLib.get_monotonic_time();
        let pending = false;
        for (const sequence of sequences) {
            if (sequence.get_completed())
                continue;
            const id = sequence.get_id();
            active.add(id);
            pending = true;
            const visible = this._windows(sequence).some(window => {
                const actor = window.get_compositor_private();
                return window.get_window_type() === Meta.WindowType.NORMAL &&
                    !window.minimized && actor?.is_mapped();
            });
            if (!visible) {
                this._seen.delete(id);
                continue;
            }
            // Require an already mapped main window for 750 ms; never expire
            // a genuinely pending launch merely because a fixed time passed.
            const firstVisible = this._seen.get(id) ?? now;
            this._seen.set(id, firstVisible);
            if (now - firstVisible >= 750_000) {
                sequence.complete();
                this._seen.delete(id);
                console.log(`Startup cursor: visible window acknowledged for ${sequence.get_application_id()}`);
            }
        }
        for (const id of this._seen.keys()) {
            if (!active.has(id))
                this._seen.delete(id);
        }
        return pending;
    }

    disable() {
        if (this._source)
            GLib.source_remove(this._source);
        this._source = 0;
        if (this._changed)
            this._notification.disconnect(this._changed);
        this._changed = 0;
        this._notification = null;
        this._seen.clear();
    }
}
