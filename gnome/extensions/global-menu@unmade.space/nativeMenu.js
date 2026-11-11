// Renders an application's own exported menu, received from the daemon as a
// JSON tree. Same presentation as the fallback menu, but activating an item
// goes back through the daemon to the app's real action.

import GLib from 'gi://GLib';

import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {MenuBarButton} from './menuBarButton.js';
import {addBarButton} from './panelBar.js';

function stripMnemonic(label) {
	return (label ?? '').replace('_', '');
}

function toOrnament(toggleType, toggleState) {
	if (toggleType === 'checkmark')
		return toggleState ? PopupMenu.Ornament.CHECK : PopupMenu.Ornament.NONE;
	if (toggleType === 'radio')
		return toggleState ? PopupMenu.Ornament.DOT : PopupMenu.Ornament.NONE;
	return undefined;
}

export class NativeMenu {
	constructor(proxy, uuid) {
		this._proxy = proxy;
		this._uuid = uuid;
		this._buttons = [];
		this._shownJson = null;
		this._pendingNodes = null;
		this._rebuildIdleId = 0;
		this._appNode = null;
		this._appName = '';
	}

	/**
	 * The items of the app's own menu, the one named after the app (Vinilo's "Vinilo", a Mac
	 * app's application menu), as a function source; null when the app exports none. The bar
	 * shows them under the app's name instead of a second button with the same name.
	 */
	appItems() {
		const node = this._appNode;
		return node ? () => this._toSpecs(node.children ?? []) : null;
	}

	hasMenus() {
		return this._buttons.length > 0 || this._appNode !== null;
	}

	setItems(nodes, appName = '') {
		this._appName = (appName ?? '').toLowerCase();
		// The daemon re-sends the tree on every menu open and window switch,
		// almost always unchanged, and rebuilding on that churns panel actors
		// and popup contents for nothing
		const json = JSON.stringify([this._appName, nodes]);
		if (json === this._shownJson) {
			this._pendingNodes = null;
			return;
		}
		// Rebuilding would destroy a menu the user is navigating, so apply
		// state updates in place and rebuild once everything is closed
		if (this._anyOpen()) {
			this._pendingNodes = nodes;
			this._applyState(nodes);
			return;
		}
		this._rebuild(nodes, json);
	}

	closeAllMenus() {
		this._buttons.forEach(btn => btn.menu.close());
	}

	clear() {
		this._shownJson = null;
		this._pendingNodes = null;
		this._buttons.forEach(btn => btn.destroy());
		this._buttons = [];
		this._appNode = null;
	}

	destroy() {
		if (this._rebuildIdleId) {
			GLib.source_remove(this._rebuildIdleId);
			this._rebuildIdleId = 0;
		}
		this.clear();
	}

	_anyOpen() {
		return this._buttons.some(btn => btn.menu.isOpen);
	}

	_rebuild(nodes, json = JSON.stringify([this._appName, nodes])) {
		this.clear();
		const topLevel = nodes.filter(n => !n.separator && stripMnemonic(n.label));
		// Only the first menu can be the app's own, as on a Mac: a later one with the app's name
		// is something else
		const first = topLevel[0];
		if (first && this._appName && stripMnemonic(first.label).toLowerCase() === this._appName)
			this._appNode = topLevel.shift();
		topLevel.forEach((node, index) => {
			const btn = new MenuBarButton(stripMnemonic(node.label), {
				// Ask the daemon for fresh toggle/enabled state on open
				onOpen: () => this._proxy.RequestMenuTree(),
				onClose: () => this._scheduleRebuildIfPending(),
			});
			// A function source builds the popup's items when it opens, not
			// here. Large apps export thousands of nodes, and building them
			// all on every tree change is what made the shell stutter.
			btn.setItems(() => this._toSpecs(node.children ?? []));
			addBarButton(`${this._uuid}-native-${index}`, btn, 1 + index);
			this._buttons.push(btn);
		});
		this._shownJson = json;
	}

	_toSpecs(nodes) {
		const specs = [];
		for (const node of nodes) {
			if (node.separator) {
				specs.push({separator: true});
				continue;
			}
			const spec = {
				label: stripMnemonic(node.label),
				enabled: node.enabled !== false,
				key: node.key,
				ornament: toOrnament(node.toggleType, node.toggleState),
			};
			if (node.children)
				spec.children = this._toSpecs(node.children);
			else if (node.key)
				spec.onActivate = () => this._proxy.ActivateMenuItem(node.key);
			specs.push(spec);
		}
		return specs;
	}

	_scheduleRebuildIfPending() {
		if (!this._pendingNodes || this._rebuildIdleId)
			return;
		// Deferred because we get here from the menu's own close signal, and
		// the button must not be destroyed mid-emission
		this._rebuildIdleId = GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
			this._rebuildIdleId = 0;
			if (this._pendingNodes && !this._anyOpen()) {
				this._rebuild(this._pendingNodes);
				// The app's own menu may have changed with the tree: the name button reads it now.
				this.onRebuilt?.();
			}
			return GLib.SOURCE_REMOVE;
		});
	}

	_applyState(nodes) {
		const stateByKey = new Map();
		const collect = list => {
			for (const node of list) {
				if (node.separator)
					continue;
				if (node.key) {
					stateByKey.set(node.key, {
						enabled: node.enabled !== false,
						ornament: toOrnament(node.toggleType, node.toggleState),
					});
				}
				if (node.children)
					collect(node.children);
			}
		};
		collect(nodes);

		for (const btn of this._buttons)
			this._applyToMenu(btn.menu, stateByKey);
	}

	_applyToMenu(menu, stateByKey) {
		for (const item of menu._getMenuItems()) {
			const state = item._globalMenuKey ? stateByKey.get(item._globalMenuKey) : null;
			if (state) {
				item.setSensitive(state.enabled);
				if (state.ornament !== undefined && item.setOrnament)
					item.setOrnament(state.ornament);
			}
			if (item.menu)
				this._applyToMenu(item.menu, stateByKey);
		}
	}
}
