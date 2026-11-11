// SPDX-License-Identifier: GPL-3.0-or-later
// Smooth hand-off from the GDM greeter to the Nodalix session.
//
// 1. The session does not register with GDM until it has a first frame, so the
//    greeter stays on screen instead of giving way to a black screen.
// 2. That first frame is the login picture with the greeter's own blur and
//    brightness, not GNOME's dark-grey system background.
// 3. When the desktop is ready the blur and the dimming are animated away and
//    the cover fades out ("desblur"), revealing the real desktop.
//
// Every step falls back to the stock behavior: a failure here must never delay
// or break a login. Set NODALIX_LOGIN_TRANSITION=0 in the session environment
// to turn all of it off.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as Main from '../ui/main.js';
import {HandoffGate, revealFrame} from './handoff.js';

// Keep in step with gnome/settings/login-background/extension.js (the greeter)
// and GNOME's unlock dialog: the first frame must match the greeter's last.
const BRIGHTNESS = 0.65;
const BLUR_RADIUS = 90;
const SHARED_BACKGROUND = '/var/lib/nodalix/login/background.jpg';
const FALLBACK_BACKGROUND = '/usr/share/backgrounds/nodalix/login/nodalix-login-fallback.jpg';

const REVEAL_TIME = 650;       // ms of desblur
const MODULES_WAIT = 1500;     // ms to wait for the desktop modules before revealing
const HANDOFF_TIMEOUT = 3000;  // ms before the session registers with GDM regardless
const GUARD_TIMEOUT = 15000;   // ms after which the cover is removed no matter what
const FIRST_PAINT_WAIT = 100;  // ms to wait for the first paint before letting the greeter go
const PAINT_WAIT = 3000;       // ms to wait for the cover to be on screen before revealing
const COVER_HOLD = 400;        // ms the cover stays fully visible once it is on screen

const State = {IDLE: 'idle', ARMED: 'armed', COVERED: 'covered', REVEALING: 'revealing', DONE: 'done'};

let state = State.IDLE;
let gate = null;
let cover = null;
let guard = 0;
let armedAt = 0;
let modulesIdle = null;
let paintId = 0;
let paintedAt = 0;
let painted = Promise.resolve();
const settleWaiters = [];

function mark(message) {
    const elapsed = armedAt ? Math.round((GLib.get_monotonic_time() - armedAt) / 1000) : 0;
    // log() always reaches the journal; GLib filters informational console output.
    log(`Nodalix login transition: ${message} (+${elapsed} ms)`);
}

function backgroundFile() {
    for (const path of [SHARED_BACKGROUND, FALLBACK_BACKGROUND]) {
        if (GLib.file_test(path, GLib.FileTest.IS_REGULAR))
            return Gio.File.new_for_path(path);
    }
    return null;
}

function enabled() {
    return !Main.sessionMode.isGreeter && Main.sessionMode.currentMode === 'user' &&
        GLib.getenv('NODALIX_LOGIN_TRANSITION') !== '0';
}

function sleep(ms) {
    let timer = 0;
    const promise = new Promise(resolve => {
        timer = setTimeout(resolve, ms);
    });
    return {promise, cancel: () => clearTimeout(timer)};
}

// Resolves when promise does, or after ms, whichever comes first.
async function waitUpTo(promise, ms) {
    const timeout = sleep(ms);
    try {
        await Promise.race([promise, timeout.promise]);
    } finally {
        timeout.cancel();
    }
}

function settle() {
    if (paintId) {
        global.stage.disconnect(paintId);
        paintId = 0;
    }
    if (guard) {
        clearTimeout(guard);
        guard = 0;
    }
    state = State.DONE;
    for (const resolve of settleWaiters.splice(0))
        resolve();
}

// True when nothing is pending: heavy start-up work may run now.
export function settled() {
    return state === State.IDLE || state === State.DONE;
}

export function whenSettled() {
    if (settled())
        return Promise.resolve();
    return new Promise(resolve => settleWaiters.push(resolve));
}

// The desktop modules tell us when they have nothing left to enable.
export function setModulesIdle(callback) {
    modulesIdle = callback;
}

class Cover {
    constructor(file) {
        this._style = `background-image: url("${file.get_uri()}"); background-size: cover;`;
        this._group = new St.Widget({
            name: 'nodalix-login-transition', reactive: false,
            layout_manager: new Clutter.FixedLayout(),
        });
        this._monitorsId = 0;
        this._scaleId = 0;
        this._progress = null;
        this._themeContext = null;
        try {
            global.stage.add_child(this._group);
            global.stage.set_child_above_sibling(this._group, null);
            this._themeContext = St.ThemeContext.get_for_stage(global.stage);
            this._monitorsId = Main.layoutManager.connect('monitors-changed', () => this._rebuild());
            this._scaleId = this._themeContext.connect('notify::scale-factor',
                () => this._apply(this._progress?.value ?? 0));
            this._rebuild();
        } catch (error) {
            this.destroy();
            throw error;
        }
    }

    _rebuild() {
        this._group.destroy_all_children();
        this._group.set_size(global.screen_width, global.screen_height);
        for (const monitor of Main.layoutManager.monitors) {
            this._group.add_child(new St.Widget({
                x: monitor.x, y: monitor.y, width: monitor.width, height: monitor.height,
                reactive: false, style: this._style, clip_to_allocation: true,
                effect: new Shell.BlurEffect({name: 'blur'}),
            }));
        }
        this._apply(this._progress?.value ?? 0);
    }

    _apply(progress) {
        const frame = revealFrame(progress, {
            radius: BLUR_RADIUS, brightness: BRIGHTNESS, scale: this._themeContext.scale_factor,
        });
        for (const widget of this._group.get_children())
            widget.get_effect('blur')?.set({brightness: frame.brightness, radius: frame.radius});
        this._group.opacity = frame.opacity;
    }

    async reveal() {
        if (!St.Settings.get().enable_animations) {
            this.destroy();
            return;
        }
        this._progress = new St.Adjustment({actor: this._group, lower: 0, upper: 1, value: 0});
        this._progress.connect('notify::value', () => this._apply(this._progress.value));
        await this._progress.easeAsync(1, {
            duration: REVEAL_TIME,
            mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
        });
        this.destroy();
    }

    destroy() {
        if (!this._group)
            return;
        if (this._monitorsId)
            Main.layoutManager.disconnect(this._monitorsId);
        if (this._scaleId)
            this._themeContext.disconnect(this._scaleId);
        this._monitorsId = this._scaleId = 0;
        this._progress = null;
        this._group.destroy();
        this._group = null;
    }
}

function abort(reason) {
    console.warn(`Nodalix login transition: ${reason}; falling back to the stock start-up`);
    try {
        cover?.destroy();
    } catch (error) {
        logError(error, 'Nodalix login transition: cover cleanup failed');
    }
    cover = null;
    gate?.release('fallback');
    settle();
}

// Called where GNOME Shell registers the session with GDM. In a normal login
// session the registration waits for the first frame (see begin()).
export function registerSession(register) {
    try {
        if (!enabled() || !backgroundFile()) {
            register();
            return;
        }
        armedAt = GLib.get_monotonic_time();
        state = State.ARMED;
        guard = setTimeout(() => abort('timed out'), GUARD_TIMEOUT);
        gate = new HandoffGate({
            register, timeout: HANDOFF_TIMEOUT,
            setTimer: (ms, callback) => setTimeout(callback, ms),
            clearTimer: id => clearTimeout(id),
            log: mark,
        });
        gate.hold();
    } catch (error) {
        logError(error, 'Nodalix login transition: could not hold the GDM registration');
        if (!gate?.release('error'))
            register();
        settle();
    }
}

// Called once the stage is about to be shown for the first time.
export function begin() {
    if (state !== State.ARMED)
        return;
    try {
        const file = backgroundFile();
        if (!file)
            throw new Error('the login picture disappeared');
        cover = new Cover(file);
        state = State.COVERED;
        mark('login picture shown');
    } catch (error) {
        logError(error, 'Nodalix login transition: could not build the cover');
        abort('no cover');
        return;
    }
    watchFirstPaint();
    releaseAfterFirstPaint();
}

// The first paint of the cover is the moment the session becomes visible. Under
// GDM the session cannot paint until GDM has switched to it, which only happens
// after the registration, so nothing before that point is ever on screen.
function watchFirstPaint() {
    painted = new Promise(resolve => {
        try {
            paintId = global.stage.connect('after-paint', () => {
                global.stage.disconnect(paintId);
                paintId = 0;
                paintedAt = GLib.get_monotonic_time();
                mark('cover painted');
                resolve();
            });
        } catch (error) {
            logError(error, 'Nodalix login transition: cannot watch the first paint');
            resolve();
        }
    });
}

// The greeter should go as soon as the cover could be painted. A session that
// is already allowed to paint reports it within a frame; one that is not gets a
// short wait, since no paint can arrive until GDM has switched to it.
function releaseAfterFirstPaint() {
    const timeout = sleep(FIRST_PAINT_WAIT);
    let released = false;
    const go = reason => {
        if (released)
            return;
        released = true;
        timeout.cancel();
        gate?.release(reason);
    };
    painted.then(() => go('first frame painted'))
        .catch(error => logError(error, 'Nodalix login transition: first paint failed'));
    timeout.promise.then(() => go('no paint yet'))
        .catch(error => logError(error, 'Nodalix login transition: hand-off timer failed'));
}

// Called before the session's own start-up animation. Waits, within limits, for
// the desktop to be complete (so the reveal does not compete with module
// start-up) and for the cover to have been on screen for a moment.
export async function beforeReveal() {
    if (state !== State.COVERED)
        return;
    try {
        await Promise.all([
            (async () => {
                try {
                    if (modulesIdle)
                        await waitUpTo(modulesIdle(), MODULES_WAIT);
                } catch (error) {
                    logError(error, 'Nodalix login transition: waiting for the desktop modules failed');
                }
            })(),
            waitUpTo(painted, PAINT_WAIT),
        ]);
        if (paintedAt) {
            const remaining = COVER_HOLD - (GLib.get_monotonic_time() - paintedAt) / 1000;
            if (remaining > 0)
                await sleep(remaining).promise;
        }
    } catch (error) {
        logError(error, 'Nodalix login transition: waiting to reveal the desktop failed');
    }
}

// Starts the desblur; the overview's own start-up animation runs underneath.
export function reveal() {
    if (state !== State.COVERED || !cover)
        return;
    state = State.REVEALING;
    mark('revealing the desktop');
    cover.reveal().catch(error => logError(error, 'Nodalix login transition: reveal failed'))
        .finally(() => {
            cover = null;
            mark('desktop revealed');
            settle();
        });
}

// Called when GNOME Shell's start-up animation ends, whatever happened before.
export function finish() {
    if (state === State.COVERED)
        reveal();
    else if (state === State.ARMED)
        abort('the stage was never shown');
}
