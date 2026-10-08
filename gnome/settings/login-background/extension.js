// SPDX-License-Identifier: GPL-3.0-or-later
import Clutter from 'gi://Clutter';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// Match GNOME 51's unlockDialog background effects.
const BRIGHTNESS = 0.65;
const BLUR_RADIUS = 90;

export default class LoginBackground extends Extension {
    enable() {
        if (!Main.sessionMode.isGreeter)
            return;
        if (Config.PACKAGE_VERSION.split('.')[0] !== '51')
            throw new Error('Nodalix login background requires GNOME 51');
        this._group = new St.Widget({name: 'nodalix-login-background',
            reactive: false, layout_manager: new Clutter.FixedLayout()});
        // Keep every login control, menu and logo above the non-interactive image.
        Main.screenShield._lockDialogGroup.insert_child_at_index(this._group, 0);
        this._themeContext = St.ThemeContext.get_for_stage(global.stage);
        this._monitorId = Main.layoutManager.connect('monitors-changed', () => this._rebuild());
        this._scaleId = this._themeContext.connect('notify::scale-factor', () => this._updateEffects());
        this._rebuild();
    }

    _rebuild() {
        this._clear();
        this._group.set_size(global.screen_width, global.screen_height);
        Main.layoutManager.monitors.forEach((monitor, monitorIndex) => {
            const widget = new St.Widget({x: monitor.x, y: monitor.y,
                width: monitor.width, height: monitor.height, reactive: false,
                style: this._backgroundStyle(),
                clip_to_allocation: true, effect: new Shell.BlurEffect({name: 'blur'})});
            this._group.add_child(widget);

        });
        this._updateEffects();
    }

    _updateEffects() {
        for (const widget of this._group.get_children())
            widget.get_effect('blur').set({brightness: BRIGHTNESS,
                radius: BLUR_RADIUS * this._themeContext.scale_factor});
    }

    _backgroundStyle() {
        const shared = '/var/lib/nodalix/login/background.jpg';
        const fallback = '/usr/share/backgrounds/nodalix/login/nodalix-login-fallback.jpg';
        const file = Gio.File.new_for_path(GLib.file_test(shared, GLib.FileTest.IS_REGULAR) ? shared : fallback);
        return `background-image: url("${file.get_uri()}"); background-size: cover; background-position: center;`;
    }

    _clear() {
        this._group?.destroy_all_children();
    }

    disable() {
        if (this._monitorId)
            Main.layoutManager.disconnect(this._monitorId);
        if (this._scaleId)
            this._themeContext.disconnect(this._scaleId);
        this._monitorId = this._scaleId = 0;
        this._clear();
        this._group?.destroy();
        this._group = this._themeContext = null;
    }
}
