/*
Copyright (C) 2025-2026  Frederyk Abryan Palinoan

This program is free software; you can redistribute it and/or
modify it under the terms of the GNU General Public License
as published by the Free Software Foundation; either version 2
of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program; if not, visit https://www.gnu.org/licenses/.
*/

import {EventEmitter} from 'resource:///org/gnome/shell/misc/signals.js';

// Settings keys
export const SHOW_ACTIVITIES_ID = 'show-activities';
export const SHOW_APP_MENU_ID = 'show-app-menu';
export const SHOW_DATE_TIME_ID = 'show-date-time';
export const AVAILABLE_INDICATORS_ID = 'available-indicators';
export const TRANSFER_INDICATORS_ID = 'transfer-indicators';
export const EXCLUDE_INDICATORS_ID = 'exclude-indicators';

// Writable cross-extension channel. Now Playing reads this instead of
// assigning onto the Main module namespace (which is not a reliable sink).
export const MMPANEL_CHANNEL = '_nodalixMultiMonitorPanels';
export const MMPANEL_BUS = '_nodalixMmPanelBus';

// Store reference to mmPanel array set by extension.js
let _mmPanelArrayRef = null;

function _bus() {
	if (!globalThis[MMPANEL_BUS])
		globalThis[MMPANEL_BUS] = new EventEmitter();
	return globalThis[MMPANEL_BUS];
}

// Helper function to set the mmPanel reference
export function setMMPanelArrayRef(mmPanelArray) {
	if (!mmPanelArray) {
		clearMMPanelArrayRef(_mmPanelArrayRef);
		return;
	}
	_mmPanelArrayRef = mmPanelArray;
	globalThis[MMPANEL_CHANNEL] = mmPanelArray;
	_bus();
}

export function clearMMPanelArrayRef(mmPanelArray = null) {
	if (mmPanelArray == null || globalThis[MMPANEL_CHANNEL] === mmPanelArray)
		delete globalThis[MMPANEL_CHANNEL];
	if (_mmPanelArrayRef === mmPanelArray || mmPanelArray == null)
		_mmPanelArrayRef = null;
}

export function notifyPanelsChanged() {
	_bus().emit('changed');
}

// Helper function to safely access mmPanel array
export function getMMPanelArray() {
	if (globalThis[MMPANEL_CHANNEL])
		return globalThis[MMPANEL_CHANNEL];
	return _mmPanelArrayRef;
}
