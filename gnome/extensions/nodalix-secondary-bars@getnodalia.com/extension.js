// SPDX-License-Identifier: GPL-3.0-or-later
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const PanelLayout = GObject.registerClass(class PanelLayout extends Clutter.LayoutManager {
    vfunc_get_preferred_width() { return [0, 0]; }
    vfunc_get_preferred_height() { return [0, 0]; }
    vfunc_allocate(container, box) {
        const children = container.get_children();
        if (children.length !== 3) return;
        const width = box.get_width(), height = box.get_height();
        const natural = children.map(child => child.get_preferred_width(height)[1]);
        const center = Math.min(natural[1], width);
        const side = Math.max(0, (width - center) / 2);
        const left = Math.min(natural[0], side), right = Math.min(natural[2], side);
        const slots = [[box.x1, left], [box.x1 + (width - center) / 2, center],
            [box.x2 - right, right]];
        children.forEach((child, i) => child.allocate(new Clutter.ActorBox({
            x1: slots[i][0], x2: slots[i][0] + slots[i][1], y1: box.y1, y2: box.y2,
        })));
    }
});

const MirrorButton = GObject.registerClass({
    Signals: {clicked: {param_types: [GObject.TYPE_UINT]}},
}, class MirrorButton extends PanelMenu.ButtonBox {
    _init(name) {
        super._init({reactive: true, can_focus: true, track_hover: true, accessible_name: name});
        const click = new Clutter.ClickGesture();
        click.set_recognize_on_press(true);
        click.connect('recognize', () => this.emit('clicked', 1));
        this.add_action(click);
        const keyboard = new Clutter.KeyController();
        keyboard.connect('key-press', () => {
            const [, key] = keyboard.get_key();
            if (![Clutter.KEY_space, Clutter.KEY_Return, Clutter.KEY_KP_Enter, Clutter.KEY_Down].includes(key))
                return Clutter.EVENT_PROPAGATE;
            this.emit('clicked', 1);
            return Clutter.EVENT_STOP;
        });
        this.add_action(keyboard);
    }
});

// GNOME 51 boundary: mirror native panel contents and borrow their existing
// menus. Never construct another Panel, QuickSettings or DateMenu controller.
export default class SecondaryBars extends Extension {
    enable() {
        if (Config.PACKAGE_VERSION.split('.')[0] !== '51')
            throw new Error('Nodalix secondary bars require GNOME 51');
        this._bars = [];
        this._signals = [];
        this._theme = St.ThemeContext.get_for_stage(global.stage);
        this._watch(Main.layoutManager, 'monitors-changed', () => this._queueRebuild());
        this._watch(this._theme, 'notify::scale-factor', () => this._queueRebuild());
        this._watch(Main.layoutManager.panelBox, 'notify::height', () => this._queueRebuild());
        for (const source of [Main.panel._leftBox, Main.panel._centerBox, Main.panel._rightBox]) {
            this._watch(source, 'child-added', () => this._queueContents());
            this._watch(source, 'child-removed', () => this._queueContents());
        }
        this._watch(Main.extensionManager, 'extension-state-changed', () => {
            this._queueContents();
            this._blurPanels();
        });
        this._watch(Main.overview, 'showing', () => this._syncOverview(true));
        this._watch(Main.overview, 'hidden', () => this._syncOverview(false));
        this._rebuild();
    }

    _watch(object, signal, callback) {
        this._signals.push([object, object.connect(signal, callback)]);
    }

    _disconnect(signals) {
        for (const [object, id] of signals) object.disconnect(id);
        signals.length = 0;
    }

    _queueRebuild() {
        if (this._rebuildId) return;
        this._rebuildId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._rebuildId = 0;
            this._rebuild();
            return GLib.SOURCE_REMOVE;
        });
    }

    _queueContents() {
        if (this._contentsId) return;
        this._contentsId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._contentsId = 0;
            this._rebuildContents();
            return GLib.SOURCE_REMOVE;
        });
    }

    _rebuild() {
        this._clear();
        const height = Main.layoutManager.panelBox.height || 32 * this._theme.scale_factor;
        for (const monitor of Main.layoutManager.monitors) {
            if (monitor.index === Main.layoutManager.primaryIndex) continue;
            const bar = new St.Widget({name: 'panelBox', x: monitor.x, y: monitor.y,
                width: monitor.width, height, layout_manager: new Clutter.BinLayout()});
            const panel = new St.Widget({name: 'panel', reactive: true,
                x_expand: true, y_expand: true, layout_manager: new PanelLayout()});
            const boxes = [Clutter.ActorAlign.START, Clutter.ActorAlign.CENTER, Clutter.ActorAlign.END]
                .map(x_align => new St.BoxLayout({x_align, y_expand: true}));
            for (const box of boxes) panel.add_child(box);
            bar.add_child(panel);
            this._bars.push({bar, panel, boxes, buttons: new Map()});
            Main.layoutManager.addChrome(bar, {affectsStruts: true, trackFullscreen: true});
        }
        this._rebuildContents();
        this._syncOverview(Main.overview.visible);
        this._blurPanels();
    }

    _blurPanels() {
        // Share the primary panel's configured cached blur pipeline, including
        // user changes. Native theme background remains if blur is disabled.
        const blur = global.blur_my_shell?._panel_blur;
        if (!blur?.enabled) return;
        for (const {panel} of this._bars) blur.maybe_blur_panel(panel);
    }

    _syncOverview(visible) {
        for (const {panel} of this._bars) {
            if (visible) panel.add_style_pseudo_class('overview');
            else panel.remove_style_pseudo_class('overview');
        }
    }

    _rebuildContents() {
        this._closeMenu();
        const sources = [Main.panel._leftBox, Main.panel._centerBox, Main.panel._rightBox];
        for (const entry of this._bars) {
            entry.buttons.clear();
            entry.boxes.forEach(box => box.destroy_all_children());
            sources.forEach((sourceBox, index) => {
                for (const actor of sourceBox.get_children()) {
                    const match = Object.entries(Main.panel.statusArea)
                        .find(([, source]) => source === actor || source.container === actor);
                    if (!match) continue;
                    const [role, source] = match;
                    if (role !== 'activities' && !source.menu) continue;
                    const child = source.get_first_child();
                    if (!child) continue;
                    const button = new MirrorButton(source.accessible_name || role);
                    const clone = new Clutter.Clone({source: child,
                        x_align: Clutter.ActorAlign.CENTER, y_align: Clutter.ActorAlign.CENTER});
                    button.add_child(clone);
                    entry.boxes[index].add_child(button.container);
                    entry.buttons.set(role, button);
                    const sync = () => {
                        // Retain the last valid size while the primary monitor
                        // hides its panel for fullscreen. Do not stretch icons.
                        if (child.width > 0 && child.height > 0)
                            clone.set_size(child.width, child.height);
                        button.visible = source.visible && child.visible;
                        button.reactive = source.reactive;
                        button.accessible_name = source.accessible_name || role;
                    };
                    child.connectObject('notify::allocation', sync, 'notify::visible', sync, button);
                    source.connectObject('notify::visible', sync, 'notify::reactive', sync,
                        'destroy', () => this._queueContents(), button);
                    button.connect('clicked', () => {
                        if (role === 'activities') Main.overview.toggle();
                        else this._toggleMenu(source.menu, button);
                    });
                    sync();
                }
            });
        }
    }

    _toggleMenu(menu, button) {
        if (this._menuState?.button === button && menu.isOpen) {
            menu.close();
            return;
        }
        this._closeMenu();
        Main.panel.menuManager.activeMenu?.close({animate: false});
        if (menu.isOpen) menu.close({animate: false});
        const pointer = menu._boxPointer;
        const state = {menu, button, source: menu.sourceActor,
            pointerSource: pointer?._sourceActor, alignment: menu._arrowAlignment};
        this._menuState = state;
        state.closedId = menu.connect('menu-closed', () => this._restoreMenu(state));
        state.destroyId = menu.connect('destroy', () => {
            state.destroyed = true;
            this._restoreMenu(state);
        });
        state.openId = menu.connect('open-state-changed', (_menu, open) => {
            if (open) button.add_style_pseudo_class('active');
            else {
                button.remove_style_pseudo_class('active');
                // Preserve the secondary arrow through the closing animation;
                // the primary button is the source of subsequent native opens.
                menu.sourceActor = state.source;
            }
        });
        menu.sourceActor = button;
        if (!menu.open()) this._restoreMenu(state);
    }

    _restoreMenu(state) {
        if (this._menuState !== state) return;
        this._menuState = null;
        const {menu, button} = state;
        menu.disconnect(state.closedId);
        menu.disconnect(state.openId);
        menu.disconnect(state.destroyId);
        menu.sourceActor = state.source;
        const pointer = menu._boxPointer;
        if (!state.destroyed && pointer?._sourceActor === button)
            pointer.setPosition(state.pointerSource || state.source, state.alignment);
        button.remove_style_pseudo_class('active');
    }

    _closeMenu() {
        const state = this._menuState;
        if (!state) return;
        state.menu.close({animate: false});
        this._restoreMenu(state);
    }

    _clear() {
        this._closeMenu();
        for (const {bar} of this._bars ?? []) {
            Main.layoutManager.removeChrome(bar);
            bar.destroy();
        }
        this._bars = [];
    }

    disable() {
        for (const key of ['_rebuildId', '_contentsId']) {
            if (this[key]) GLib.source_remove(this[key]);
            this[key] = 0;
        }
        this._disconnect(this._signals);
        this._clear();
        this._theme = null;
    }
}
