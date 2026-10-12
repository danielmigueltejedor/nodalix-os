// SPDX-License-Identifier: GPL-3.0-or-later
import Clutter from 'gi://Clutter';
import GnomeDesktop from 'gi://GnomeDesktop';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// All interaction with GNOME private chrome is isolated here. No Panel.Panel,
// dateMenu, QuickSettings, actor reparenting or statusArea duplication.
function attachBar(bar) {
    Main.layoutManager.addChrome(bar, {affectsStruts: true, trackFullscreen: true});
}
function detachBar(bar) {
    Main.layoutManager.removeChrome(bar);
    bar.destroy();
}

export default class SecondaryBars extends Extension {
    enable() {
        if (Config.PACKAGE_VERSION.split('.')[0] !== '51')
            throw new Error('Nodalix secondary bars require GNOME 51');
        this._bars = [];
        this._clock = new GnomeDesktop.WallClock();
        this._clockId = this._clock.connect('notify::clock', () => this._refreshClock());
        this._monitorsId = Main.layoutManager.connect('monitors-changed', () => this._rebuild());
        this._theme = St.ThemeContext.get_for_stage(global.stage);
        this._scaleId = this._theme.connect('notify::scale-factor', () => this._rebuild());
        this._heightId = Main.layoutManager.panelBox.connect('notify::height', () => this._rebuild());
        this._rebuild();
    }

    _rebuild() {
        this._clear();
        const height = Main.layoutManager.panelBox.height || 32 * this._theme.scale_factor;
        for (const monitor of Main.layoutManager.monitors) {
            if (monitor.index === Main.layoutManager.primaryIndex)
                continue;
            const bar = new St.BoxLayout({style_class: 'nodalix-secondary-bar',
                x: monitor.x, y: monitor.y, width: monitor.width, height,
                reactive: true});
            const activities = new St.Button({label: 'Nodalix', style_class: 'panel-button',
                can_focus: true, accessible_name: 'Activities'});
            activities.connect('clicked', () => Main.overview.toggle());
            const clock = new St.Label({text: this._clock.clock,
                x_expand: true, x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER});
            // An inert spacer balances Activities without cloning a menu actor.
            const spacer = new St.Widget({width: 96 * this._theme.scale_factor});
            bar.add_child(activities); bar.add_child(clock); bar.add_child(spacer);
            bar.add_effect(new Shell.BlurEffect({mode: Shell.BlurMode.BACKGROUND,
                brightness: 0.75, radius: 30 * this._theme.scale_factor}));
            this._bars.push({bar, clock});
            attachBar(bar);
        }
    }

    _refreshClock() {
        for (const {clock} of this._bars)
            clock.text = this._clock.clock;
    }
    _clear() {
        for (const {bar} of this._bars ?? [])
            detachBar(bar);
        this._bars = [];
    }
    disable() {
        if (this._monitorsId) Main.layoutManager.disconnect(this._monitorsId);
        if (this._scaleId) this._theme.disconnect(this._scaleId);
        if (this._heightId) Main.layoutManager.panelBox.disconnect(this._heightId);
        if (this._clockId) this._clock.disconnect(this._clockId);
        this._monitorsId = this._scaleId = this._heightId = this._clockId = 0;
        this._clear();
        this._clock = this._theme = null;
    }
}
