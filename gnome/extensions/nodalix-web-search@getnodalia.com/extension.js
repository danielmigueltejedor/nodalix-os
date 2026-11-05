import Gio from 'gi://Gio';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as ParentalControls from 'resource:///org/gnome/shell/misc/parentalControlsManager.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const LEGACY_ID = 'com.nodalix.WebSearch.desktop';
const HELPER = '/usr/lib/nodalix/web-search-provider';

function query(terms) {
    return [...terms.join(' ').trim()].slice(0, 512).join('');
}

class WebProvider {
    constructor(browser, parentalControls) {
        this.id = 'nodalix-web-search';
        this.appInfo = browser;
        this.parentalControls = parentalControls;
        this.canLaunchSearch = true;
    }

    getInitialResultSet(terms) {
        const value = query(terms);
        return value && this.parentalControls.shouldShowApp(this.appInfo) ? [value] : [];
    }

    getSubsearchResultSet(_previous, terms) {
        return this.getInitialResultSet(terms);
    }

    getResultMetas(ids) {
        return ids.map(id => ({
            id,
            name: id,
            createIcon: size => new St.Icon({icon_name: 'web-browser-symbolic', icon_size: size}),
        }));
    }

    filterResults(results, maximum) {
        return results.slice(0, maximum);
    }

    activateResult(value) {
        const browser = Gio.AppInfo.get_default_for_uri_scheme('https');
        if (value && browser && this.parentalControls.shouldShowApp(browser))
            Gio.Subprocess.new([HELPER, '--search', value], Gio.SubprocessFlags.NONE);
    }

    launchSearch(terms) {
        this.activateResult(query(terms));
    }
}

export default class NodalixWebSearch extends Extension {
    enable() {
        this.controller = Main.overview.searchController;
        this.parentalControls = ParentalControls.getDefault();
        this.settings = new Gio.Settings({schema_id: 'org.gnome.desktop.search-providers'});
        this.appMonitor = Gio.AppInfoMonitor.get();
        this.signals = [
            [this.settings, 'changed::disabled'],
            [this.settings, 'changed::disable-external'],
            [this.appMonitor, 'changed'],
            [this.parentalControls, 'app-filter-changed'],
        ].map(([object, signal]) => [object, object.connect(signal, () => this.refresh())]);
        this.refresh();
    }

    refresh() {
        this.removeProvider();
        if (this.settings.get_boolean('disable-external') ||
            this.settings.get_strv('disabled').includes(LEGACY_ID))
            return;
        const browser = Gio.AppInfo.get_default_for_uri_scheme('https');
        if (!browser || !this.parentalControls.shouldShowApp(browser))
            return;
        this.provider = new WebProvider(browser, this.parentalControls);
        this.controller.addProvider(this.provider);
    }

    removeProvider() {
        if (this.provider)
            this.controller.removeProvider(this.provider);
        this.provider = null;
    }

    disable() {
        for (const [object, id] of this.signals ?? [])
            object.disconnect(id);
        this.signals = [];
        this.removeProvider();
    }
}
