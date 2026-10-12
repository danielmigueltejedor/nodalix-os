// Parses GTK-style accelerator strings ("<Ctrl><Shift>N") and sends them to
// the focused window through a Clutter virtual keyboard device. Keyvals, not
// raw evdev scancodes, so a shortcut lands on the right key whatever the
// keyboard layout is.
//
// parseAccelerator() is shared with shortcutStore.js, which also runs in the
// preferences process, and extensions must not import Clutter there. So the
// keyvals below are numeric literals (the same values as Clutter.KEY_*, which
// are X11 keysyms) and Clutter itself is imported lazily on the first
// synthesis, which only ever happens in the shell process.

import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

let debug = false;

export function setDebug(enabled) {
	debug = enabled;
}

function logDebug(msg) {
	if (debug)
		console.log(`[global-menu-keysynth] ${msg}`);
}

// Modifier token (lowercased, without <>) to left-modifier keyval. <Meta> is
// deliberately not accepted: it means different things on different setups.
const MODIFIER_KEYVALS = {
	'ctrl': 0xffe3,    // Control_L
	'control': 0xffe3,
	'primary': 0xffe3,
	'shift': 0xffe1,   // Shift_L
	'alt': 0xffe9,     // Alt_L
	'super': 0xffeb,   // Super_L
};

const NAMED_KEYVALS = {
	'f1': 0xffbe, 'f2': 0xffbf, 'f3': 0xffc0, 'f4': 0xffc1,
	'f5': 0xffc2, 'f6': 0xffc3, 'f7': 0xffc4, 'f8': 0xffc5,
	'f9': 0xffc6, 'f10': 0xffc7, 'f11': 0xffc8, 'f12': 0xffc9,
	'return': 0xff0d,
	'escape': 0xff1b,
	'tab': 0xff09,
	'backspace': 0xff08,
	'delete': 0xffff,
	'home': 0xff50,
	'end': 0xff57,
	'page_up': 0xff55,
	'page_down': 0xff56,
	'left': 0xff51,
	'up': 0xff52,
	'right': 0xff53,
	'down': 0xff54,
	'space': 0x020,
	'exclam': 0x021,
	'quotedbl': 0x022,
	'numbersign': 0x023,
	'dollar': 0x024,
	'percent': 0x025,
	'ampersand': 0x026,
	'apostrophe': 0x027,
	'parenleft': 0x028,
	'parenright': 0x029,
	'asterisk': 0x02a,
	'plus': 0x02b,
	'comma': 0x02c,
	'minus': 0x02d,
	'period': 0x02e,
	'slash': 0x02f,
	'colon': 0x03a,
	'semicolon': 0x03b,
	'less': 0x03c,
	'equal': 0x03d,
	'greater': 0x03e,
	'question': 0x03f,
	'at': 0x040,
	'bracketleft': 0x05b,
	'backslash': 0x05c,
	'bracketright': 0x05d,
	'asciicircum': 0x05e,
	'underscore': 0x05f,
	'grave': 0x060,
	'braceleft': 0x07b,
	'bar': 0x07c,
	'braceright': 0x07d,
	'asciitilde': 0x07e,
	'print': 0xff61,
	'insert': 0xff63,
	'menu': 0xff67,
	'kp_0': 0xffb0, 'kp_1': 0xffb1, 'kp_2': 0xffb2, 'kp_3': 0xffb3,
	'kp_4': 0xffb4, 'kp_5': 0xffb5, 'kp_6': 0xffb6, 'kp_7': 0xffb7,
	'kp_8': 0xffb8, 'kp_9': 0xffb9,
	'kp_decimal': 0xffae, 'kp_add': 0xffab, 'kp_subtract': 0xffad,
	'kp_multiply': 0xffaa, 'kp_divide': 0xffaf, 'kp_enter': 0xff8d,
};

// Chords land as one burst otherwise, and an app waiting on a prefix key
// cannot tell them apart.
const CHORD_GAP_US = 1000;

/**
 * "<Ctrl><Shift>N" to [{modifiers: [keyval, ...], keyval}], or null if the
 * string is not a valid accelerator.
 *
 * Space-separated chords are a sequence: "<Ctrl>x s" is Emacs' C-x s and
 * ": w Return" is Vim's :w, for apps whose menus live behind a prefix key
 * rather than a single accelerator. The usual one-chord accelerator is just a
 * one-element sequence.
 */
export function parseAccelerator(accel) {
	if (typeof accel !== 'string' || accel === '')
		return null;

	const chords = [];
	for (const part of accel.split(' ')) {
		const chord = parseChord(part);
		if (!chord)
			return null;
		chords.push(chord);
	}
	return chords;
}

function parseChord(accel) {
	if (accel === '')
		return null;

	const modifiers = [];
	let rest = accel;
	let match;
	while ((match = rest.match(/^<([^<>]+)>/))) {
		const keyval = MODIFIER_KEYVALS[match[1].toLowerCase()];
		if (keyval === undefined)
			return null;
		if (!modifiers.includes(keyval))
			modifiers.push(keyval);
		rest = rest.slice(match[0].length);
	}

	let keyval;
	if (rest.length === 1) {
		// Shift is expressed by the <Shift> modifier, never by an uppercase
		// keyval, so a printable character is always lowercased here
		const code = rest.toLowerCase().charCodeAt(0);
		if (code < 0x21 || code > 0x7e)
			return null;
		keyval = code;
	} else {
		keyval = NAMED_KEYVALS[rest.toLowerCase()];
		if (keyval === undefined)
			return null;
	}
	return {modifiers, keyval};
}

// Imported on first synthesis, shell process only. See the module comment.
let clutterPromise = null;

function getClutter() {
	clutterPromise ??= import('gi://Clutter').then(mod => mod.default);
	return clutterPromise;
}

/**
 * Parse and synthesize an accelerator. Returns false and logs a warning if
 * the accelerator is unparseable, true if synthesis was dispatched.
 */
export function sendShortcut(accel, targetWindow = global.display.focus_window, allowSystem = false) {
	if (!allowSystem && isSystemShortcut(accel, true)) {
		console.warn(`[global-menu-keysynth] Shortcut reserved by the system: ${accel}`);
		return false;
	}
	const chords = parseAccelerator(accel);
	if (!chords) {
		console.warn(`[global-menu-keysynth] Unparseable accelerator: ${accel}`);
		return false;
	}
	logDebug(`Synthesizing ${accel}`);
	getClutter()
		.then(Clutter => {
			// Keep the target and wait until the popup has released its input grab.
			GLib.timeout_add(GLib.PRIORITY_DEFAULT, 120, () => {
				if (!targetWindow || !global.get_window_actors().some(a => a.meta_window === targetWindow))
					return GLib.SOURCE_REMOVE;
				targetWindow.activate(global.get_current_time());
				GLib.timeout_add(GLib.PRIORITY_DEFAULT, 60, () => {
					if (global.display.focus_window === targetWindow) {
						try { synthesize(Clutter, chords); } catch (e) { console.error(`[global-menu-keysynth] ${e}`); }
					}
					return GLib.SOURCE_REMOVE;
				});
				return GLib.SOURCE_REMOVE;
			});
		})
		.catch(e => console.error(`[global-menu-keysynth] Synthesis failed for "${accel}": ${e}`));
	return true;
}

function synthesize(Clutter, chords) {
	const seat = global.stage.get_context().get_backend().get_default_seat();
	const dev = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
	activeDevices.add(dev);
	let t = GLib.get_monotonic_time();
	for (const {modifiers, keyval} of chords) {
		for (const mod of modifiers)
			dev.notify_keyval(t += 10, mod, Clutter.KeyState.PRESSED);
		dev.notify_keyval(t += 10, keyval, Clutter.KeyState.PRESSED);
		dev.notify_keyval(t += 10, keyval, Clutter.KeyState.RELEASED);
		for (const mod of [...modifiers].reverse())
			dev.notify_keyval(t += 10, mod, Clutter.KeyState.RELEASED);
		t += CHORD_GAP_US;
	}
	GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
		activeDevices.delete(dev);
		return GLib.SOURCE_REMOVE;
	});
}

// Keep the device alive while Mutter processes its queued events.
const activeDevices = new Set();

// Avoid delivering an application's menu command to a compositor binding.
let reservedCache = new Set();
let reservedCacheTime = 0;
function acceleratorIdentity(accel) {
 const parsed = parseAccelerator(accel);
 return parsed?.length === 1 ? JSON.stringify([parsed[0].keyval,[...parsed[0].modifiers].sort()]) : null;
}
export function isSystemShortcut(accel, refresh = false) {
 const now = GLib.get_monotonic_time();
 if (refresh || now - reservedCacheTime > 500000) {
  reservedCache = new Set(); reservedCacheTime = now;
  const source = Gio.SettingsSchemaSource.get_default();
  const schemas = ['org.gnome.desktop.wm.keybindings','org.gnome.mutter.keybindings',
   'org.gnome.mutter.wayland.keybindings','org.gnome.shell.keybindings',
   'org.gnome.settings-daemon.plugins.media-keys'];
  const add = value => { const id = acceleratorIdentity(value); if(id) reservedCache.add(id); };
  for (const id of schemas) {
   const schema = source.lookup(id,true); if (!schema) continue;
   const settings = Gio.Settings.new_full(schema,null,null);
   for (const key of schema.list_keys()) {
    const value = settings.get_value(key);
    if (value.get_type_string() === 'as') value.deepUnpack().forEach(add);
    else if (value.get_type_string() === 's') add(value.deepUnpack());
   }
   if (id === 'org.gnome.settings-daemon.plugins.media-keys') {
    const customSchema = source.lookup(id+'.custom-keybinding',true);
    if (customSchema) for (const path of settings.get_strv('custom-keybindings'))
     add(Gio.Settings.new_full(customSchema,null,path).get_string('binding'));
   }
  }
 }
 const key = acceleratorIdentity(accel);
 return key !== null && reservedCache.has(key);
}
