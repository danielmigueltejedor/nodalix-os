import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import GdkPixbuf from 'gi://GdkPixbuf';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import ShakeGesture from './gesture.js';

const MAGNIFICATION = 2.6;
const HOLD_MS = 220;
const BUTTONS = Clutter.ModifierType.BUTTON1_MASK | Clutter.ModifierType.BUTTON2_MASK |
    Clutter.ModifierType.BUTTON3_MASK;
const CURSOR_DIRECTORY = '/usr/share/icons/Nodalix/cursors_scalable/default';

export default class ShakePointerExtension extends Extension {
    enable() {
        this._gesture = new ShakeGesture();
        this._settings = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        this._tracker = global.backend.get_cursor_tracker();
        this._backend = global.stage.get_context().get_backend();
        this._seat = this._backend.get_default_seat();
        this._laters = global.compositor.get_laters();
        this._later = 0;
        this._timeout = 0;
        // Track the compositor pointer even when a client owns the events.
        this._positionChanged = this._tracker.connect('position-invalidated', () => {
            if (this._later)
                return;
            this._later = this._laters.add(Meta.LaterType.BEFORE_REDRAW, () => {
                this._later = 0;
                this._updatePointer();
                return GLib.SOURCE_REMOVE;
            });
        });
    }

    _updatePointer() {
        const [point, modifiers] = this._tracker.get_pointer();
        if (modifiers & BUTTONS) {
            this._gesture.reset();
            this._hide();
            return;
        }

        const now = GLib.get_monotonic_time() / 1000;
        const shaken = this._gesture.push(point.x, point.y, now);
        if (shaken || (this._overlay && this._gesture.fast)) {
            if (!this._overlay && !this._show())
                return;
            if (this._shrinking) {
                this._overlay.remove_all_transitions();
                this._overlay.set_scale(1, 1);
                this._shrinking = false;
            }
            this._deadline = now + HOLD_MS;
            if (!this._timeout)
                this._expireAfter(HOLD_MS);
        }

        if (this._overlay)
            this._overlay.set_position(point.x - this._hotX, point.y - this._hotY);
    }

    _expireAfter(delay) {
        this._timeout = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delay, () => {
            this._timeout = 0;
            const remaining = Math.ceil(this._deadline - GLib.get_monotonic_time() / 1000);
            if (remaining > 0) {
                this._expireAfter(remaining);
            } else {
                this._shrinking = true;
                this._overlay.ease({
                    scale_x: 1 / MAGNIFICATION,
                    scale_y: 1 / MAGNIFICATION,
                    duration: 140,
                    mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                    onComplete: () => this._hide(),
                });
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    _loadArtwork() {
        const cursorSize = this._settings.get_int('cursor-size');
        const size = cursorSize * MAGNIFICATION;
        // Render once at the largest monitor's physical size. Moving between
        // monitors can then reuse the texture without upscaling or decoding.
        const scale = Math.max(1, ...Main.layoutManager.monitors.map(
            ({index}) => global.display.get_monitor_scale(index)));
        const pixels = Math.ceil(size * scale);
        if (this._artwork?.pixels === pixels && this._artwork.cursorSize === cursorSize)
            return this._artwork;

        const [ok, bytes] = GLib.file_get_contents(`${CURSOR_DIRECTORY}/metadata.json`);
        if (!ok)
            throw new Error('Nodalix cursor metadata could not be loaded');
        const [frame] = JSON.parse(new TextDecoder().decode(bytes));
        const pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
            `${CURSOR_DIRECTORY}/${frame.filename}`, pixels, pixels, true);
        const width = pixbuf.get_width();
        const height = pixbuf.get_height();
        const texture = Cogl.Texture2D.new_from_data(this._backend.get_cogl_context(),
            width, height, Cogl.PixelFormat.RGBA_8888, pixbuf.get_rowstride(), pixbuf.get_pixels());
        this._artwork = {texture, pixels, cursorSize, size, width: width * size / pixels,
            height: height * size / pixels, hotX: frame.hotspot_x * size / frame.nominal_size,
            hotY: frame.hotspot_y * size / frame.nominal_size};
        return this._artwork;
    }

    _show() {
        if (!this._tracker.get_pointer_visible() || !this._tracker.get_sprite())
            return false;

        let artwork;
        try {
            artwork = this._loadArtwork();
        } catch (error) {
            console.error('Shake to Find Pointer could not load its vector cursor', error);
            return false;
        }
        const {texture, width, height, hotX, hotY} = artwork;
        this._hotX = hotX;
        this._hotY = hotY;
        this._overlay = new Clutter.Actor({reactive: false, width, height});
        this._overlay.content = Clutter.TextureContent.new_from_texture(texture, null);
        this._overlay.set_pivot_point(hotX / width, hotY / height);
        Main.uiGroup.add_child(this._overlay);
        // Match GNOME's magnifier: keep the seat focused when hiding the sprite.
        this._seat.inhibit_unfocus();
        this._tracker.inhibit_cursor_visibility();
        return true;
    }

    _hide() {
        if (this._timeout) {
            GLib.Source.remove(this._timeout);
            this._timeout = 0;
        }
        if (this._overlay) {
            const overlay = this._overlay;
            this._overlay = null;
            this._shrinking = false;
            overlay.destroy();
            this._tracker.uninhibit_cursor_visibility();
            this._seat.uninhibit_unfocus();
        }
    }

    disable() {
        this._tracker.disconnect(this._positionChanged);
        if (this._later)
            this._laters.remove(this._later);
        this._hide();
        this._artwork = null;
        this._tracker = null;
        this._backend = null;
        this._seat = null;
        this._laters = null;
        this._settings = null;
        this._gesture = null;
    }
}
