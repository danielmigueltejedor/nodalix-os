import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Layout from './layout.js';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';
import * as OverviewControls from 'resource:///org/gnome/shell/ui/overviewControls.js';


const AutoTilingToggle = GObject.registerClass(
class AutoTilingToggle extends QuickSettings.QuickToggle {
    constructor(extensionObject) {
        super({
            title: 'Mosaico automático',
            iconName: 'view-grid-symbolic',
            toggleMode: true,
        });

        this._settings = extensionObject.getSettings();

        this._settings.bind(
            'enabled',
            this,
            'checked',
            Gio.SettingsBindFlags.DEFAULT
        );
    }
});


const AutoTilingIndicator = GObject.registerClass(
class AutoTilingIndicator extends QuickSettings.SystemIndicator {
    constructor(extensionObject) {
        super();

        this._settings = extensionObject.getSettings();

        this._indicator = this._addIndicator();
        this._indicator.icon_name = 'view-grid-symbolic';

        this._settings.bind(
            'enabled',
            this._indicator,
            'visible',
            Gio.SettingsBindFlags.DEFAULT
        );

        this.quickSettingsItems.push(
            new AutoTilingToggle(extensionObject)
        );
    }

    destroy() {
        this.quickSettingsItems.forEach(item => item.destroy());
        super.destroy();
    }
});


export default class NodalixAutoTilingExtension extends Extension {
    enable() {
        this._settings = this.getSettings();

        this._floating = new Set();
        this._windowSignals = new Map();
        this._firstFrameSignals = new Map();
        this._signals = [];
        this._retileSource = 0;
        this._closeCancellable = new Gio.Cancellable();
        this._closeDebug = Gio.File.new_for_path(`${GLib.get_user_cache_dir()}/nodalix-close-debug`);
        this._groups = new Map();
        this._windowGroups = new Map();
        this._grab = null;
        this._applyingLayout = false;

        this._indicator = new AutoTilingIndicator(this);

        Main.panel.statusArea.quickSettings.addExternalIndicator(
            this._indicator
        );

        this._connect(
            this._settings,
            'changed::enabled',
            () => {
                this._cancelGrab();
                if (this._settings.get_boolean('enabled'))
                    this._scheduleRetile();
            }
        );

        this._connect(
            this._settings,
            'changed::gap',
            () => this._scheduleRetile()
        );

        this._connect(
            this._settings,
            'changed::max-windows',
            () => this._scheduleRetile()
        );

        this._connect(
            global.display,
            'window-created',
            (_display, win) => {
                this._watchWindow(win);
                this._watchFirstFrame(win);
                this._scheduleRetile();
            }
        );

        this._connect(
            global.display,
            'window-entered-monitor',
            () => this._scheduleRetile()
        );

        this._connect(
            global.display,
            'window-left-monitor',
            () => this._scheduleRetile()
        );

        this._connect(
            global.display,
            'workareas-changed',
            () => this._scheduleRetile()
        );

        this._connect(
            global.display,
            'grab-op-begin',
            (_display, win, operation, sprite) => this._onGrabBegin(win, operation, sprite)
        );

        this._connect(
            global.display,
            'grab-op-end',
            (_display, win) => this._onGrabEnd(win)
        );

        this._connect(
            global.workspace_manager,
            'active-workspace-changed',
            () => this._scheduleRetile()
        );

        this._connect(
            global.workspace_manager,
            'workspace-added',
            () => this._scheduleRetile()
        );

        this._connect(
            global.workspace_manager,
            'workspace-removed',
            () => this._scheduleRetile()
        );

        for (const win of global.display.list_all_windows())
            this._watchWindow(win);

        Main.wm.addKeybinding(
            'toggle-floating',
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL,
            () => this._toggleFocusedFloating()
        );

Main.wm.addKeybinding(
'toggle-launcher',
this._settings,
Meta.KeyBindingFlags.NONE,
Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
() => this._toggleLauncher()
);

        const closeAction = Main.wm.addKeybinding(
            'close-window', this._settings,
            Meta.KeyBindingFlags.NON_MASKABLE | Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.NORMAL,
            () => this._closeWindow(global.display.get_focus_window(), global.get_current_time())
        );
        if (this._closeDebug?.query_exists(null))
            console.log(`Nodalix close shortcut registered: ${closeAction}`);

        if (this._settings.get_boolean('enabled'))
            this._scheduleRetile();
    }


    disable() {
        this._cancelGrab();
        this._groups?.clear();
        this._windowGroups?.clear();

        Main.wm.removeKeybinding('close-window');
        this._closeCancellable?.cancel();
        this._closeCancellable = null;

        Main.wm.removeKeybinding('toggle-floating');
        Main.wm.removeKeybinding('toggle-launcher');

        if (this._retileSource) {
            GLib.source_remove(this._retileSource);
            this._retileSource = 0;
        }

        for (const [object, id] of this._signals ?? []) {
            try {
                object.disconnect(id);
            } catch (_) {
            }
        }

        this._signals = [];

        for (const win of this._firstFrameSignals?.keys() ?? [])
            this._disconnectFirstFrame(win);

        for (const [win, ids] of this._windowSignals ?? []) {
            for (const id of ids) {
                try {
                    win.disconnect(id);
                } catch (_) {
                }
            }
        }

        this._windowSignals?.clear();

        this._indicator?.destroy();

        this._indicator = null;
        this._floating = null;
        this._windowSignals = null;
        this._firstFrameSignals = null;
        this._settings = null;
    }


    _closeWindow(win, timestamp) {
        if (!win)
            return;
        if (this._closeDebug?.query_exists(null))
            console.log(`Nodalix close request: class=${win.get_wm_class()} gtk=${win.get_gtk_application_id()}`);

        const isGhostty = win.get_gtk_application_id() === 'com.mitchellh.ghostty' ||
            win.get_wm_class()?.toLowerCase() === 'ghostty' ||
            win.get_wm_class() === 'com.mitchellh.ghostty';
        const closeNormally = () => {
            if (win.get_compositor_private() && (isGhostty || win.can_close()))
                win.delete(timestamp);
        };

        if (!isGhostty) {
            closeNormally();
            return;
        }

        // GTK exports an action on each window. Use the focused window's
        // unique bus name and object path, never the application-wide quit action.
        const busName = win.get_gtk_unique_bus_name();
        const windowPath = win.get_gtk_window_object_path();
        const cancellable = this._closeCancellable;
        if (this._closeDebug?.query_exists(null))
            console.log(`Nodalix Ghostty close route: bus=${busName} path=${windowPath}`);
        if (!busName || !windowPath || !cancellable) {
            closeNormally();
            return;
        }

        Gio.DBus.session.call(
            busName,
            windowPath,
            'org.gtk.Actions',
            'Activate',
            new GLib.Variant('(sava{sv})', ['close', [], {}]),
            null,
            Gio.DBusCallFlags.NONE,
            2000,
            cancellable,
            (connection, result) => {
                try {
                    connection.call_finish(result);
                } catch (error) {
                    if (!cancellable.is_cancelled()) {
                        console.warn(`Nodalix: Ghostty window close failed: ${error.message}`);
                        closeNormally();
                    }
                }
            }
        );
    }


    _connect(object, signal, callback) {
        this._signals.push([
            object,
            object.connect(signal, callback),
        ]);
    }


    _watchWindow(win) {
        if (!win || this._windowSignals.has(win))
            return;

        const ids = [];

        ids.push(
            win.connect(
                'shown',
                () => this._scheduleRetile()
            )
        );

        ids.push(
            win.connect(
                'notify::minimized',
                () => this._scheduleRetile()
            )
        );

        ids.push(
            win.connect(
                'workspace-changed',
                () => this._scheduleRetile()
            )
        );

        ids.push(
            win.connect(
                'unmanaged',
                () => {
                    if (this._grab?.win === win)
                        this._cancelGrab();
                    this._disconnectFirstFrame(win);
                    this._floating.delete(win.get_stable_sequence());
                    this._windowSignals.delete(win);
                    this._scheduleRetile();
                }
            )
        );

        this._windowSignals.set(win, ids);
    }


    _watchFirstFrame(win) {
        // window-created runs before a Wayland client has drawn its content.
        // Reflow again once Mutter has the first complete frame and size hints.
        const actor = win.get_compositor_private();
        if (!actor || this._firstFrameSignals.has(win))
            return;

        const id = actor.connect('first-frame', () => {
            this._disconnectFirstFrame(win);
            this._scheduleRetile();
        });
        const destroyId = actor.connect('destroy', () => {
            // Destroy can precede MetaWindow::unmanaged; never access a disposed actor.
            this._firstFrameSignals.delete(win);
        });
        this._firstFrameSignals.set(win, [actor, id, destroyId]);
    }


    _disconnectFirstFrame(win) {
        const signal = this._firstFrameSignals?.get(win);
        if (!signal)
            return;

        const [actor, id, destroyId] = signal;
        try {
            actor.disconnect(id);
            actor.disconnect(destroyId);
        } catch (_) {
            // The compositor may already have destroyed an unmapped actor.
        }
        this._firstFrameSignals.delete(win);
    }


    _scheduleRetile() {
        if (!this._settings?.get_boolean('enabled'))
            return;

        if (this._retileSource || this._grab)
            return;

        this._retileSource = GLib.idle_add(
            GLib.PRIORITY_DEFAULT_IDLE,
            () => {
                this._retileSource = 0;
                this._retileAll();

                return GLib.SOURCE_REMOVE;
            }
        );
    }


    _retileAll() {
        if (this._grab || !this._settings?.get_boolean('enabled'))
            return;

        const manager = global.workspace_manager;

        const workspaces = manager.get_n_workspaces();
        const monitors = global.display.get_n_monitors();
        const activeWorkspaces = new Set();

        for (let workspaceIndex = 0;
             workspaceIndex < workspaces;
             workspaceIndex++) {

            const workspace =
                manager.get_workspace_by_index(workspaceIndex);
            activeWorkspaces.add(workspace);

            for (let monitor = 0;
                 monitor < monitors;
                 monitor++) {

                this._retileGroup(
                    workspace,
                    monitor
                );
            }
        }
        for (const [workspace, groups] of this._groups) {
            for (const [monitor, group] of groups) {
                if (!activeWorkspaces.has(workspace) || monitor >= monitors) {
                    for (const id of Layout.ids(group.tree)) {
                        if (this._windowGroups.get(id) === group)
                            this._windowGroups.delete(id);
                    }
                    groups.delete(monitor);
                }
            }
            if (groups.size === 0)
                this._groups.delete(workspace);
        }
    }


    _getGroup(workspace, monitor) {
        let monitors = this._groups.get(workspace);
        if (!monitors) {
            monitors = new Map();
            this._groups.set(workspace, monitors);
        }
        if (!monitors.has(monitor)) {
            monitors.set(monitor, {workspace, monitor, tree: null,
                windows: new Map(), sizes: new Map(), rects: new Map()});
        }
        return monitors.get(monitor);
    }

    _updateGroupArea(group) {
        group.gap = Math.max(0, this._settings.get_int('gap'));
        group.area = Layout.inset(group.workspace.get_work_area_for_monitor(group.monitor), group.gap);
    }

    _minimumSize(win) {
        const [known, width, height] = win.get_min_size?.() ?? [false, 0, 0];
        return {width: Math.max(64, known ? width : 0), height: Math.max(64, known ? height : 0)};
    }

    _fits(tree, group) {
        const min = Layout.minimum(tree, group.gap, group.sizes);
        return min.width <= group.area.width && min.height <= group.area.height;
    }

    _insertWindow(group, id) {
        if (!group.tree) {
            group.tree = {id};
            return;
        }
        const focus = global.display.get_focus_window?.()?.get_stable_sequence();
        const current = Layout.ids(group.tree);
        const preferred = current.includes(focus) ? focus : current.at(-1);
        const rects = Layout.rectangles(group.tree, group.area, group.gap, group.sizes).leaves;
        // Split the largest region first: 1, 2, 3 and 4 windows naturally form
        // a full area, halves, a half plus quarters, and four quarters.
        const candidates = [...current].sort((a, b) => {
            const ra = rects.get(a), rb = rects.get(b);
            return rb.width * rb.height - ra.width * ra.height || (a === preferred ? -1 : b === preferred ? 1 : 0);
        });
        for (const target of candidates) {
            const rect = rects.get(target);
            const side = rect.width >= rect.height ? 'right' : 'bottom';
            for (const direction of [side, side === 'right' ? 'bottom' : 'right']) {
                const tree = Layout.insert(JSON.parse(JSON.stringify(group.tree)), id, target, direction);
                if (this._fits(tree, group)) {
                    group.tree = tree;
                    return;
                }
            }
        }
        // A client whose minimum size cannot fit stays usable as a floating window.
    }

    _retileGroup(workspace, monitor) {
        const group = this._getGroup(workspace, monitor);
        this._updateGroupArea(group);
        const limit = this._settings.get_int('max-windows');
        let windows = workspace.list_windows()
            .filter(win => this._isTileable(win, workspace, monitor))
            .sort((a, b) => a.get_stable_sequence() - b.get_stable_sequence());
        if (limit > 0)
            windows = windows.slice(0, limit);
        group.windows = new Map(windows.map(win => [win.get_stable_sequence(), win]));
        group.sizes = new Map(windows.map(win => [win.get_stable_sequence(), this._minimumSize(win)]));
        for (const id of Layout.ids(group.tree)) {
            if (!group.windows.has(id)) {
                group.tree = Layout.remove(group.tree, id);
                if (this._windowGroups.get(id) === group)
                    this._windowGroups.delete(id);
            }
        }
        // If the monitor shrinks, make room without forcing clients below their minima.
        while (Layout.ids(group.tree).length > 1 && !this._fits(group.tree, group)) {
            const id = Layout.ids(group.tree).at(-1);
            group.tree = Layout.remove(group.tree, id);
            if (this._windowGroups.get(id) === group)
                this._windowGroups.delete(id);
        }
        for (const id of group.windows.keys()) {
            if (!Layout.ids(group.tree).includes(id))
                this._insertWindow(group, id);
        }
        for (const id of Layout.ids(group.tree))
            this._windowGroups.set(id, group);
        this._applyGroup(group);
    }

    _applyGroup(group, skip = null) {
        group.rects = Layout.rectangles(group.tree, group.area, group.gap, group.sizes).leaves;
        this._applyingLayout = true;
        try {
            for (const [id, rect] of group.rects) {
                const win = group.windows.get(id);
                if (win && win !== skip)
                    this._applyRect(win, rect);
            }
        } finally {
            this._applyingLayout = false;
        }
    }

    _resizeEdges(operation) {
        const keys = [
            ['N', ['top']], ['S', ['bottom']], ['W', ['left']], ['E', ['right']],
            ['NW', ['top', 'left']], ['NE', ['top', 'right']],
            ['SW', ['bottom', 'left']], ['SE', ['bottom', 'right']],
        ];
        for (const [suffix, edges] of keys) {
            if (operation === Meta.GrabOp[`RESIZING_${suffix}`] ||
                operation === Meta.GrabOp[`KEYBOARD_RESIZING_${suffix}`])
                return edges;
        }
        return [];
    }

    _onGrabBegin(win, operation, sprite) {
        if (!win || !this._settings.get_boolean('enabled'))
            return;
        operation &= ~1024; // Mutter WINDOW_FLAG_UNCONSTRAINED is not exported in GrabOp.
        const moving = operation === Meta.GrabOp.MOVING || operation === Meta.GrabOp.KEYBOARD_MOVING;
        const edges = this._resizeEdges(operation);
        if (!moving && !edges.length)
            return;
        const id = win.get_stable_sequence();
        const group = this._windowGroups.get(id);
        if (!group || this._floating.has(id))
            return;
        this._cancelGrab();
        if (this._retileSource) {
            GLib.source_remove(this._retileSource);
            this._retileSource = 0;
        }
        this._grab = {win, id, group, moving, edges, sprite, signals: [], source: 0, drop: null};
        if (moving) {
            this._preview = new St.Widget({reactive: false, can_focus: false,
                style: 'background-color: rgba(23,147,209,0.22); border: 2px solid #1793d1; border-radius: 10px;'});
            Main.uiGroup.add_child(this._preview);
            this._preview.hide();
            this._grab.source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 16, () => {
                this._updateDropPreview();
                return GLib.SOURCE_CONTINUE;
            });
        } else {
            const update = () => this._updateResize();
            this._grab.signals.push(win.connect('size-changed', update));
            this._grab.signals.push(win.connect('position-changed', update));
        }
    }

    _updateResize() {
        const grab = this._grab;
        if (!grab || grab.moving || this._applyingLayout)
            return;
        Layout.resize(grab.group.tree, grab.id, grab.edges, grab.win.get_frame_rect(),
            grab.group.area, grab.group.gap, grab.group.sizes);
        this._applyGroup(grab.group);
    }

    _dropAtPointer() {
        const grab = this._grab;
        const coords = grab.sprite?.get_coords?.();
        const [x, y] = coords ? [coords.x, coords.y] : global.get_pointer();
        const workspace = grab.win.get_workspace();
        for (let monitor = 0; monitor < global.display.get_n_monitors(); monitor++) {
            const group = this._getGroup(workspace, monitor);
            this._updateGroupArea(group);
            const workArea = workspace.get_work_area_for_monitor(monitor);
            if (!Layout.contains(workArea, x, y))
                continue;
            const dropX = Math.max(group.area.x, Math.min(group.area.x + group.area.width - 1, x));
            const dropY = Math.max(group.area.y, Math.min(group.area.y + group.area.height - 1, y));
            const drop = Layout.dropTarget(group.rects, group.area, dropX, dropY, grab.id);
            if (drop)
                return {group, ...drop};
            if (!group.tree)
                return {group, target: null, side: 'right', root: true};
        }
        return null;
    }

    _dropTree(drop) {
        const grab = this._grab;
        const tree = JSON.parse(JSON.stringify(drop.group.tree));
        if (drop.group === grab.group)
            return Layout.move(tree, grab.id, drop.target, drop.side);
        return Layout.insert(tree, grab.id, drop.target, drop.side === 'center' ? 'right' : drop.side);
    }

    _updateDropPreview() {
        const drop = this._dropAtPointer();
        this._grab.drop = null;
        if (!drop) {
            this._preview.hide();
            return;
        }
        const sizes = new Map(drop.group.sizes);
        sizes.set(this._grab.id, this._minimumSize(this._grab.win));
        const tree = this._dropTree(drop);
        const min = Layout.minimum(tree, drop.group.gap, sizes);
        if (min.width > drop.group.area.width || min.height > drop.group.area.height) {
            this._preview.hide();
            return;
        }
        const rect = Layout.rectangles(tree, drop.group.area, drop.group.gap, sizes).leaves.get(this._grab.id);
        if (!rect) {
            this._preview.hide();
            return;
        }
        this._grab.drop = drop;
        this._preview.set_position(rect.x, rect.y);
        this._preview.set_size(rect.width, rect.height);
        this._preview.show();
    }

    _onGrabEnd(win) {
        const grab = this._grab;
        if (!grab || (win && win !== grab.win)) {
            this._scheduleRetile();
            return;
        }
        if (grab.moving) {
            this._updateDropPreview();
            const drop = grab.drop;
            if (drop) {
                const tree = this._dropTree(drop);
                if (drop.group !== grab.group) {
                    grab.group.tree = Layout.remove(grab.group.tree, grab.id);
                    grab.win.move_to_monitor(drop.group.monitor);
                }
                drop.group.tree = tree;
                this._windowGroups.set(grab.id, drop.group);
            }
        } else {
            this._updateResize();
        }
        this._cancelGrab();
        this._scheduleRetile();
    }

    _cancelGrab() {
        const grab = this._grab;
        this._grab = null;
        if (grab) {
            if (grab.source)
                GLib.source_remove(grab.source);
            for (const id of grab.signals) {
                try { grab.win.disconnect(id); } catch (_) {}
            }
        }
        this._preview?.destroy();
        this._preview = null;
    }


    _isTileable(win, workspace, monitor) {
        if (
            win.get_window_type() !==
            Meta.WindowType.NORMAL
        )
            return false;

        if (
            win.minimized ||
            win.is_fullscreen() ||
            win.is_skip_taskbar()
        )
            return false;

        if (win.is_always_on_all_workspaces())
            return false;

        if (
            win.get_workspace() !== workspace ||
            win.get_monitor() !== monitor
        )
            return false;

        if (
            win.get_transient_for() ||
            win.is_attached_dialog()
        )
            return false;

        if (
            (!win.allows_move() || !win.allows_resize()) &&
            !win.is_maximized()
        )
            return false;

        /*
         * Always-on-top windows normally behave like
         * PiP/tools, so leave them floating.
         */
        if (win.is_above())
            return false;

        const title =
            (win.get_title() ?? "")
                .toLowerCase();

        if (
            title.includes("picture-in-picture") ||
            title.includes("picture in picture")
        )
            return false;

        return !this._floating.has(
            win.get_stable_sequence()
        );
    }


    _applyRect(win, rect) {
        try {
            if (win.is_maximized())
                win.unmaximize();
            const frame = win.get_frame_rect?.();
            if (frame && frame.x === rect.x && frame.y === rect.y &&
                frame.width === rect.width && frame.height === rect.height)
                return;

            win.move_resize_frame(
                false,
                rect.x,
                rect.y,
                Math.max(64, rect.width),
                Math.max(64, rect.height)
            );
        } catch (error) {
            console.error(
                `Nodalix Auto Tiling: ${error}`
            );
        }
    }


    _toggleLauncher() {
const controls = Main.overview._overview.controls;

// Escritorio / Overview -> Apps
// Apps -> Escritorio
if (
Main.overview.visible &&
controls.dash.showAppsButton.checked
) {
Main.overview.hide();
return;
}

Main.overview.show(
OverviewControls.ControlsState.APP_GRID
);
}


    _toggleFocusedFloating() {
        const win =
            global.display.get_focus_window();

        if (!win)
            return;

        const id =
            win.get_stable_sequence();

        if (this._floating.has(id))
            this._floating.delete(id);
        else
            this._floating.add(id);

        this._scheduleRetile();
    }
}
