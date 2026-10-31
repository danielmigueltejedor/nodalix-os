// GNOME 51 compatibility boundary. Keep the existing native notification model.
import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as MessageList from 'resource:///org/gnome/shell/ui/messageList.js';
import {InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

export const API_VERSION = 1;
const pinned = notification => notification?._nodalixLiveCall === true;
const hasCall = group => [...(group._notificationToMessage?.keys() ?? [])].some(pinned);

export class NativeNotifications {
    enable() {
        try { this._enable(); }
        catch (error) { this.disable(); throw error; }
    }

    _enable() {
        this._injections = new InjectionManager();
        this._signals = [];
        this._date = Main.panel.statusArea.dateMenu;
        this._list = this._date._messageList;
        this._view = this._list._messageView;
        this._oldStyle = this._list._scrollView.get_style();
        this._oldLayout = [this._list.y_expand, this._list.y_align,
            this._list._scrollView.y_expand, this._list._scrollView.y_align,
            this._list._scrollView.hscrollbar_policy, this._list._scrollView.vscrollbar_policy];
        this._list.y_expand = false;
        this._list.y_align = Clutter.ActorAlign.START;
        this._list._scrollView.y_expand = false;
        this._list._scrollView.y_align = Clutter.ActorAlign.START;
        const daemon = Main.notificationDaemon._fdoNotificationDaemon;
        const owner = this;
        // Attach the existing backend's live-call hint to the native object.
        this._injections.overrideMethod(Object.getPrototypeOf(daemon), 'NotifyAsync', original => function (params, invocation) {
            const id = params[1] && this._notifications.has(params[1]) ? params[1] : this._nextNotificationId;
            const hint = params[6]['x-nodalix-phone-call'];
            const live = hint?.deepUnpack?.() ?? hint;
            const result = original.call(this, params, invocation);
            const notification = this._notifications.get(id);
            if (notification) {
                notification._nodalixLiveCall = live === true && notification.resident;
                owner._refresh();
            }
            return result;
        });
        this._injections.overrideMethod(MessageList.NotificationMessage.prototype, 'canClose', original => function () {
            return !pinned(this.notification) && original.call(this);
        });
        this._injections.overrideMethod(MessageList.NotificationMessage.prototype, 'close', original => function () {
            if (!pinned(this.notification)) original.call(this);
        });
        this._injections.overrideMethod(MessageList.NotificationMessageGroup.prototype, 'canClose', original => function () {
            return !hasCall(this) && original.call(this);
        });
        this._injections.overrideMethod(MessageList.NotificationMessageGroup.prototype, 'close', original => function () {
            if (!hasCall(this)) return original.call(this);
            for (const [notification, message] of this._notificationToMessage)
                if (!pinned(notification)) message.close();
        });
        // Keep media and newly arriving critical notifications below live calls.
        for (const method of ['_addMessageAtIndex', '_moveMessage']) {
            this._injections.overrideMethod(MessageList.MessageView.prototype, method, original => function (message, index) {
                const count = this.messages.filter(m => m !== message && hasCall(m)).length;
                return original.call(this, message, hasCall(message) ? 0 : Math.max(index, count));
            });
        }
        this._injections.overrideMethod(MessageList.MessageView.prototype, 'clear', original => function () {
            for (const group of this.messages.filter(hasCall)) group.close();
            return original.call(this);
        });
        this._connect(Main.layoutManager, 'monitors-changed', () => this._height());
        this._connect(global.display, 'workareas-changed', () => this._height());
        this._connect(St.ThemeContext.get_for_stage(global.stage), 'notify::scale-factor', () => this._height());
        this._connect(this._list._scrollView.vadjustment, 'notify::upper', () => this._height());
        this._connect(this._list._scrollView, 'notify::width', () => this._height());
        this._connect(this._view, 'notify::empty', () => this._height());
        this._connect(this._date.menu, 'open-state-changed', (_menu, open) => { if (open) this._refresh(); });
        this._refresh();
    }

    _connect(object, signal, callback) {
        this._signals.push([object, object.connect(signal, callback)]);
    }

    _height() {
        const monitor = Main.layoutManager.findMonitorForActor(this._date) ?? Main.layoutManager.primaryMonitor;
        if (!monitor) return;
        const area = global.workspace_manager.get_active_workspace().get_work_area_for_monitor(monitor.index);
        const scale = St.ThemeContext.get_for_stage(global.stage).scale_factor;
        // Leave room for the popup padding and the native controls outside the scroller.
        const height = Math.max(1, Math.floor(area.height * 0.65 / scale));
        if (this._maxHeight !== height) {
            this._list._scrollView.set_style(`${this._oldStyle ?? ''}; max-height: ${height}px;`);
            this._maxHeight = height;
        }
        // The calendar's BoxLayout can allocate more than a CSS maximum.
        // Request a content-sized viewport, capped in stage coordinates.
        const content = this._view.get_preferred_height(this._list._scrollView.width || -1)[1];
        const requested = Math.min(height * scale, content);
        if (Math.abs(this._list._scrollView.height - requested) > 0.5)
            this._list._scrollView.set_height(requested);
        this._list._scrollView.hscrollbar_policy = St.PolicyType.NEVER;
        this._list._scrollView.vscrollbar_policy = St.PolicyType.AUTOMATIC;
    }

    _refresh() {
        this._height();
        for (const group of this._view.messages) {
            for (const [notification, message] of group._notificationToMessage ?? []) {
                message._header.closeButton.visible = !pinned(notification);
                if (pinned(notification)) {
                    // Stop an earlier native reorder animation before moving this item.
                    const item = message.get_parent();
                    item.remove_all_transitions();
                    item.set_scale(1, 1);
                    group.set_child_at_index(item, 0);
                }
            }
            if (hasCall(group)) {
                const item = group.get_parent();
                item.remove_all_transitions();
                item.set_scale(1, 1);
                this._view.messages.splice(this._view.messages.indexOf(group), 1);
                this._view.messages.unshift(group);
            }
        }
        this._view.notify('can-clear');
        this._view.queue_relayout();
    }

    disable() {
        for (const [object, id] of this._signals ?? []) object.disconnect(id);
        this._signals = [];
        this._injections?.clear();
        this._list?._scrollView.set_style(this._oldStyle);
        this._list?._scrollView.set_height(-1);
        if (this._list && this._oldLayout) {
            [this._list.y_expand, this._list.y_align,
                this._list._scrollView.y_expand, this._list._scrollView.y_align,
                this._list._scrollView.hscrollbar_policy, this._list._scrollView.vscrollbar_policy] = this._oldLayout;
        }
        for (const group of this._view?.messages ?? [])
            for (const message of group._notificationToMessage?.values() ?? [])
                message._header.closeButton.visible = message.canClose();
        this._view?.notify('can-clear');
        this._view = this._list = this._date = null;
    }
}
