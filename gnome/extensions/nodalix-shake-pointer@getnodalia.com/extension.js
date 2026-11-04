import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import ShakeGesture from './gesture.js';

const MAGNIFICATION = 2.6;
const BUTTONS = Clutter.ModifierType.BUTTON1_MASK | Clutter.ModifierType.BUTTON2_MASK |
    Clutter.ModifierType.BUTTON3_MASK;

export default class ShakePointerExtension extends Extension {
    enable() {
        this._gesture = new ShakeGesture();
        this._tracker = global.backend.get_cursor_tracker();
        this._timeout = 0;
        this._motion = global.stage.connect('captured-event', (_actor, event) => {
            if (event.type() === Clutter.EventType.MOTION)
                this._onMotion(event);
            return Clutter.EVENT_PROPAGATE;
        });
        this._cursorChanged = this._tracker.connect('cursor-changed', () => {
            if (this._overlay)
                this._refreshSprite();
        });
    }

    _onMotion(event) {
        if (event.get_state() & BUTTONS) {
            this._gesture.reset();
            this._hide();
            return;
        }
        const [x, y] = event.get_coords();
        this._point = [x, y];
        if (this._overlay)
            this._move();
        if (this._gesture.push(x, y, GLib.get_monotonic_time() / 1000))
            this._show();
    }

    _show() {
        if (!this._overlay) {
            const sprite = this._tracker.get_sprite();
            if (!sprite || !this._tracker.get_pointer_visible())
                return;
            this._overlay = new Clutter.Actor({reactive: false});
            this._refreshSprite(sprite);
            Main.uiGroup.add_child(this._overlay);
            this._tracker.inhibit_cursor_visibility();
        }
        this._overlay.remove_all_transitions();
        this._overlay.set_scale(MAGNIFICATION, MAGNIFICATION);
        if (this._timeout)
            GLib.Source.remove(this._timeout);
        this._timeout = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 650, () => {
            this._timeout = 0;
            this._overlay.ease({
                scale_x: 1,
                scale_y: 1,
                duration: 160,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                onComplete: () => this._hide(),
            });
            return GLib.SOURCE_REMOVE;
        });
    }

    _refreshSprite(sprite = this._tracker.get_sprite()) {
        if (!sprite) {
            this._hide();
            return;
        }
        const scale = this._tracker.get_scale();
        const [x, y] = this._tracker.get_hot();
        this._hotspot = [x * scale, y * scale];
        this._overlay.content = Clutter.TextureContent.new_from_texture(sprite, null);
        this._overlay.set_size(sprite.get_width() * scale, sprite.get_height() * scale);
        this._overlay.set_pivot_point(x / sprite.get_width(), y / sprite.get_height());
        this._move();
    }

    _move() {
        const [x, y] = this._point;
        const [hotX, hotY] = this._hotspot;
        this._overlay.set_position(x - hotX, y - hotY);
    }

    _hide() {
        if (this._timeout) {
            GLib.Source.remove(this._timeout);
            this._timeout = 0;
        }
        if (this._overlay) {
            const overlay = this._overlay;
            this._overlay = null;
            overlay.destroy();
            this._tracker.uninhibit_cursor_visibility();
        }
    }

    disable() {
        global.stage.disconnect(this._motion);
        this._tracker.disconnect(this._cursorChanged);
        this._hide();
        this._tracker = null;
        this._gesture = null;
    }
}
