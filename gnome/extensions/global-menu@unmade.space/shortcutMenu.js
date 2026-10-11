// Renders a curated shortcut mapping from shortcutStore.js. Same
// presentation as the other two menu sources, but activating an item
// synthesizes the application's real keyboard shortcut through keySynth.js.

import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Shell from 'gi://Shell';

import {MenuBarButton} from './menuBarButton.js';
import {addBarButton} from './panelBar.js';
import {sendShortcut, isSystemShortcut} from './keySynth.js';

// When an item activates the shell still holds the input grab, and the
// target window may not have keyboard focus back yet, so synthesizing
// immediately delivers the keystroke to the shell instead of the app.
const SYNTH_DELAY_MS = 100;

export class ShortcutMenu {
	constructor(uuid) {
		this._uuid = uuid;
		this._buttons = [];
		this._mapping = null;
		this._timeoutIds = new Set();
	}

	/** Build buttons for the mapping, or clear with null. */
	setMapping(mapping, targetWindow = global.display.focus_window) {
		this._targetWindow = targetWindow;
		// The daemon re-sends empty trees periodically. Rebuilding for the
		// mapping already shown would close a menu the user is navigating.
		if (mapping === this._mapping && (mapping === null || this._buttons.length > 0))
			return;
		this.clear();
		this._mapping = mapping;
		if (!mapping)
			return;
		mapping.menus.forEach((menu, index) => {
			const btn = new MenuBarButton(menu.label);
			btn.setItems(this._toSpecs(menu.items));
			addBarButton(`${this._uuid}-shortcut-${index}`, btn, 1 + index);
			this._buttons.push(btn);
		});
	}

	hasMenus() {
		return this._buttons.length > 0;
	}

	closeAllMenus() {
		this._buttons.forEach(btn => btn.menu.close());
	}

	clear() {
		this._mapping = null;
		this._buttons.forEach(btn => btn.destroy());
		this._buttons = [];
	}

	destroy() {
		for (const id of this._timeoutIds)
			GLib.source_remove(id);
		this._timeoutIds.clear();
		this.clear();
	}

	_toSpecs(items) {
		return items.map(item => {
			if (item.separator)
				return {separator: true};
			if (item.items)
				return {label: item.label, children: this._toSpecs(item.items)};
			const direct = this._directItem(item.label);
			if (direct) return direct;
			if (item.shortcut) {
				if (isSystemShortcut(item.shortcut)) return {label: item.label, enabled: false};
				return {label: item.label, onActivate: () => this._fireShortcut(item.shortcut)};
			}
			// Placeholder: the mapping records the item but no usable
			// accelerator for it, so show it greyed out rather than lie
			return {label: item.label, enabled: false};
		});
	}

 _directItem(label) {
  const win=this._targetWindow;
  const app=win ? Shell.WindowTracker.get_default().get_window_app(win) : null;
  const info=app?.get_app_info();
  if (!win) return null;
  if (label === 'Close Window')
   return {label,enabled:win.can_close(),onActivate:()=>this._targetWindow?.delete(global.get_current_time())};
  if (!info) return null;
  if (['New Window','Nueva ventana independiente'].includes(label)) {
   const actions=info.list_actions();
   const action=actions.includes('new-blank-window') ? 'new-blank-window' : actions.includes('new-window') ? 'new-window' : null;
   if (action) return {label,onActivate:()=>info.launch_action(action,global.create_app_launch_context(0,-1))};
  }
  if (label === 'Open File' && ['zen.desktop','app.zen_browser.zen.desktop'].includes(app.get_id()))
   return {label,onActivate:()=>Gio.Subprocess.new([GLib.build_filenamev([GLib.get_home_dir(),'.local/bin/nodalix-global-menu-open-file']),app.get_id()],Gio.SubprocessFlags.NONE)};
  return null;
 }

	_fireShortcut(accel) {
		const targetWindow = this._targetWindow;
		const id = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SYNTH_DELAY_MS, () => {
			this._timeoutIds.delete(id);
			sendShortcut(accel, targetWindow);
			return GLib.SOURCE_REMOVE;
		});
		this._timeoutIds.add(id);
	}
}
