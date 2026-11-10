// SPDX-License-Identifier: GPL-3.0-or-later
// Pure logic of the GDM hand-off (no GNOME imports so it can be unit tested).

/**
 * Holds the "RegisterSession" call to GDM until the session has something
 * to show.
 *
 * GDM keeps the greeter on screen until the session registers. Upstream GNOME
 * Shell registers before its first frame, so the greeter goes away while the
 * new compositor still has nothing to draw: that is the black gap. The gate
 * releases the registration exactly once, either when the session is ready or
 * when the safety timeout expires (a login must never hang).
 */
export class HandoffGate {
    constructor({register, timeout, setTimer, clearTimer, log = () => {}}) {
        this._register = register;
        this._timeout = timeout;
        this._setTimer = setTimer;
        this._clearTimer = clearTimer;
        this._log = log;
        this._timer = null;
        this._held = false;
        this._done = false;
    }

    get released() {
        return this._done;
    }

    hold() {
        if (this._held || this._done)
            return;
        this._held = true;
        this._timer = this._setTimer(this._timeout, () => {
            this._timer = null;
            this.release('timeout');
        });
    }

    release(reason) {
        if (this._done)
            return false;
        this._done = true;
        if (this._timer !== null) {
            this._clearTimer(this._timer);
            this._timer = null;
        }
        this._log(`registering session with GDM (${reason})`);
        this._register();
        return true;
    }
}

/**
 * Colour of the blurred login picture at progress p (0 = identical to the
 * greeter, 1 = the plain wallpaper). Returned values are applied to a
 * Shell.BlurEffect and to the opacity of the whole cover.
 */
export function revealFrame(p, {radius, brightness, scale = 1}) {
    const progress = Math.min(1, Math.max(0, p));
    // The cover starts to fade a third of the way in, while the blur is still
    // clearing, so a wallpaper that differs from the login picture cross-fades
    // instead of popping.
    const fade = Math.min(1, Math.max(0, (progress - 0.35) / 0.65));
    return {
        radius: Math.round(radius * scale * (1 - progress)),
        brightness: brightness + (1 - brightness) * progress,
        opacity: Math.round(255 * (1 - fade)),
    };
}
