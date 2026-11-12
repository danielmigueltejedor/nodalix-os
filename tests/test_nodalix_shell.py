"""Native Nodalix Shell overlay tests (does not modify the host)."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import subprocess
import shutil
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "prepare_nodalix_shell", ROOT / "tools/prepare-nodalix-shell.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def fixture(path: Path, version: str = "51.1") -> None:
    (path / "js/ui").mkdir(parents=True, exist_ok=True)
    (path / "js/extensions").mkdir(exist_ok=True)
    (path / "js/extensions/sharedInternals.js").write_text(
        MODULE.GETTEXT_DETECTION + "\n"
        "        const path = extensionLine.slice(extensionLine.indexOf(basePath));\n")
    (path / "js/meson.build").write_text("subdir('misc')\n")
    (path / "meson.build").write_text(
        f"project('gnome-shell', 'c',\n  version: '{version}',\n)\n"
        "mutter_api_version = '51'\n")
    (path / "js/ui/main.js").write_text(
        "import * as Panel from './panel.js';\n"
        "export async function start() {\n"
        "    await _initializeUI();\n" + MODULE.MAIN_LAYOUT +
        MODULE.MAIN_REGISTER + "}\n")
    (path / "js/ui/layout.js").write_text(
        MODULE.LAYOUT_IMPORT + "class LayoutManager {\n"
        "    async _doStartupAnimation() {\n        await this._loadBackground();\n\n"
        + MODULE.LAYOUT_FIRST_FRAME +
        "    }\n" + MODULE.LAYOUT_SESSION_ANIMATION +
        "            await Main.overview.runStartupAnimation();\n        }\n    }\n"
        "    _startupAnimationComplete() {\n" + MODULE.LAYOUT_COMPLETE + "    }\n}\n")
    (path / "js/ui/extensionSystem.js").write_text(
        "import * as Main from './main.js';\n"
        "        return this._extensions.get(uuid);\n"
        "            const uuid = info.get_name();\n"
        "            const existing = this.lookup(uuid);\n"
        "        return extensions.filter(item => !disabledExtensions.includes(item));\n")
    (path / "js/js-resources.gresource.xml").write_text(
        "<gresources>\n  <gresource prefix=\"/org/gnome/shell\">\n"
        "    <file>ui/main.js</file>\n"
        "  </gresource>\n</gresources>\n")


class NativeShellOverlayTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.source = Path(self.temp.name)
        self.overlay = ROOT / "gnome/nodalix-shell"
        fixture(self.source)

    def test_native_phone_link_is_bundled_and_legacy_settings_are_preserved(self):
        MODULE.prepare(self.source, self.overlay)
        main = (self.source / "js/ui/main.js").read_text()
        resources = (self.source / "js/js-resources.gresource.xml").read_text()
        extensions = (self.source / "js/ui/extensionSystem.js").read_text()
        self.assertIn("Nodalix.start({panel, sessionMode, extensionManager, notifyError})", main)
        self.assertLess(main.index("global.connect('shutdown', Nodalix.stop)"),
                        main.index("layoutManager = new Layout.LayoutManager()"))
        self.assertIn("nodalix/main.js", resources)
        self.assertIn("nodalix/connect.js", resources)
        self.assertIn("nodalix-connect@getnodalia.com", (self.source / "js/nodalix/modules.js").read_text())
        self.assertIn("!NodalixModules.managesModule(item)", extensions)
        self.assertIn("NodalixModules.lookupModule(uuid)", extensions)
        self.assertNotIn("set_strv", extensions)
        self.assertIn("bus_watch_name", (self.source / "js/nodalix/connect.js").read_text())
        self.assertIn("sessionMode.isGreeter", (self.source / "js/nodalix/main.js").read_text())

    def test_session_registration_and_first_frame_are_wired_to_the_transition(self):
        MODULE.prepare(self.source, self.overlay)
        main = (self.source / "js/ui/main.js").read_text()
        layout = (self.source / "js/ui/layout.js").read_text()
        self.assertIn("Nodalix.registerSession(() => LoginManager.registerSessionWithGDM());", main)
        self.assertNotIn("\n    LoginManager.registerSessionWithGDM();", main)
        self.assertIn("import * as NodalixTransition from '../nodalix/transition.js';", layout)
        # Cover first, then show the stage; the reveal is scheduled before the stock
        # animation (which is not held back); clean up last.
        order = [layout.index(text) for text in (
            "NodalixTransition.begin();", "global.stage.show();",
            "NodalixTransition.reveal();",
            "await Main.overview.runStartupAnimation();",
            "NodalixTransition.finish();", "this._systemBackground.destroy();")]
        self.assertEqual(order, sorted(order))
        for name in ("handoff.js", "transition.js"):
            self.assertTrue((self.source / "js/nodalix" / name).is_file())
            self.assertIn(f"nodalix/{name}", (self.source / "js/js-resources.gresource.xml").read_text())

    def test_bundled_desktop_modules_keep_their_upstream_ids(self):
        MODULE.prepare(self.source, self.overlay)
        manifest = self.source / "js/nodalix/modules.json"
        registry = json.loads(manifest.read_text())
        module_ids = registry["bundled"] + registry["desktopPackages"]
        self.assertEqual(len(set(module_ids)), len(module_ids))
        self.assertIn("blur-my-shell@aunetx", module_ids)
        self.assertIn("tiling-assistant@leleat-on-github", module_ids)
        self.assertIn("global-menu@unmade.space", module_ids)
        self.assertIn("hanabi-extension@jeffshee.github.io", registry["desktopPackages"])
        self.assertIn("glocalsend@donnybeelo.github.com", registry["desktopPackages"])
        self.assertIn("nowplaying@epogonii.github.io", registry["bundled"])
        for uuid in set(module_ids) - {
            "hanabi-extension@jeffshee.github.io", "nowplaying@epogonii.github.io",
            "nodalix-login-background@getnodalia.com",
        }:
            path = ROOT / "gnome/extensions" / uuid
            self.assertTrue((path / "extension.js").is_file())
            self.assertEqual(json.loads((path / "metadata.json").read_text())["uuid"], uuid)

    def test_shutdown_uses_native_teardown_and_regular_disable_restores_desktop(self):
        script = r"""
const fs = require('fs');
const assert = require('assert');
const source = fs.readFileSync(process.argv[1], 'utf8');
const definition = source.slice(source.indexOf('export class DesktopModules'))
    .replace('export class', 'return class');
const records = new Map();
const Component = new Function('records', 'ExtensionState', 'logError', definition)(
    records, {INACTIVE: 2}, error => { throw error; });
for (const stopped of [false, true]) {
    const calls = [];
    const manager = {
        stopped,
        extensionManager: {
            _unloadExtensionStylesheet: () => calls.push('style'),
            emit: () => calls.push('signal'),
        },
    };
    const record = {uuid: 'test', stateObj: {
        disable: () => calls.push('disable'),
        shutdown: () => calls.push('shutdown'),
    }};
    records.set(record.uuid, record);
    Component.prototype.disable.call(manager, record);
    assert.deepStrictEqual(calls, [stopped ? 'shutdown' : 'disable', 'style', 'signal']);
    assert.strictEqual(records.size, 0);
    assert.strictEqual(record.enabled, false);
}
"""
        subprocess.run(["node", "-e", script, str(self.overlay / "modules.js")], check=True)

    def test_gdm_registration_is_held_once_and_always_released(self):
        script = r"""
const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const copy = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'handoff-')), 'handoff.mjs');
fs.copyFileSync(process.argv[1], copy);
import(copy).then(({HandoffGate, revealFrame}) => {
    const make = () => {
        const log = [], timers = [];
        const gate = new HandoffGate({
            register: () => log.push('register'), timeout: 3000,
            setTimer: (ms, fn) => { timers.push({ms, fn, live: true}); return timers.length - 1; },
            clearTimer: id => { timers[id].live = false; },
            log: message => log.push(message),
        });
        return {gate, log, timers};
    };
    // Normal path: nothing is registered until the session is ready, and only once.
    let {gate, log, timers} = make();
    gate.hold(); gate.hold();
    assert.deepStrictEqual(log, []);
    assert.strictEqual(timers.length, 1);
    assert.strictEqual(timers[0].ms, 3000);
    assert.ok(gate.release('ready'));
    assert.ok(!gate.release('again'));
    assert.deepStrictEqual(log.filter(x => x === 'register'), ['register']);
    assert.strictEqual(timers[0].live, false);
    // Safety path: a session that never gets a frame still registers.
    ({gate, log, timers} = make());
    gate.hold();
    timers[0].fn();
    assert.deepStrictEqual(log.filter(x => x === 'register'), ['register']);
    assert.ok(gate.released);
    assert.ok(!gate.release('late'));
    // The first frame equals the greeter's; the last one is the plain wallpaper.
    const options = {radius: 90, brightness: 0.65, scale: 2};
    assert.deepStrictEqual(revealFrame(0, options), {radius: 180, brightness: 0.65, opacity: 255});
    assert.deepStrictEqual(revealFrame(1, options), {radius: 0, brightness: 1, opacity: 0});
    let previous = revealFrame(0, options);
    for (let p = 0.05; p <= 1.0001; p += 0.05) {
        const frame = revealFrame(p, options);
        assert.ok(frame.radius <= previous.radius && frame.brightness >= previous.brightness &&
            frame.opacity <= previous.opacity);
        previous = frame;
    }
    assert.deepStrictEqual(revealFrame(-3, options), revealFrame(0, options));
    assert.deepStrictEqual(revealFrame(7, options), revealFrame(1, options));
}).catch(error => { console.error(error); process.exit(1); });
"""
        subprocess.run(["node", "-e", script, str(self.overlay / "handoff.js")], check=True)

    def test_first_frame_matches_the_greeter_and_never_blocks_a_login(self):
        transition = (self.overlay / "transition.js").read_text()
        greeter = (ROOT / "gnome/settings/login-background/extension.js").read_text()
        for constant in ("BRIGHTNESS = 0.65", "BLUR_RADIUS = 90"):
            self.assertIn(constant, transition)
            self.assertIn(constant, greeter)
        for path in ("/var/lib/nodalix/login/background.jpg",
                     "/usr/share/backgrounds/nodalix/login/nodalix-login-fallback.jpg"):
            self.assertIn(path, transition)
            self.assertIn(path, greeter)
        self.assertIn("NODALIX_LOGIN_TRANSITION", transition)
        # Timestamps must reach the journal (console.info is filtered by GLib)
        # and the greeter waits for the first paint, within a bounded time.
        self.assertIn("log(`Nodalix login transition:", transition)
        self.assertNotIn("console.info", transition)
        self.assertIn("'after-paint'", transition)
        self.assertRegex(transition, r"FIRST_PAINT_WAIT = \d+")
        # The reveal waits for the cover to be on screen: a session under GDM
        # cannot paint before the registration, so animating earlier is invisible.
        self.assertRegex(transition, r"COVER_HOLD = \d+")
        self.assertRegex(transition, r"PAINT_WAIT = \d+")
        self.assertIn("mark('cover painted')", transition)
        self.assertNotIn("beforeReveal", transition)
        # ... and until GDM has handed the screen over: the greeter outlives the
        # registration, so painting (or animating) earlier is never seen.
        self.assertIn("getCurrentSessionProxy()", transition)
        self.assertIn("'g-properties-changed'", transition)
        self.assertRegex(transition, r"DISPLAY_WAIT = \d+")
        self.assertRegex(transition, r"STARTUP_WAIT = \d+")
        self.assertIn("waitUpTo(active, DISPLAY_WAIT)", transition)
        self.assertIn("mark('session active')", transition)
        # A stalled start-up must release GDM and remove the cover on its own.
        self.assertRegex(transition, r"HANDOFF_TIMEOUT = \d+")
        self.assertRegex(transition, r"GUARD_TIMEOUT = \d+")
        self.assertIn("isGreeter", transition)

    def test_reveal_waits_for_the_screen_to_be_handed_over(self):
        script = r"""
const assert = require('assert');
const fs = require('fs');
const source = fs.readFileSync(process.argv[1], 'utf8')
    .replace(/^import .*$/gm, '').replaceAll('export ', '')
    .replace(/COVER_HOLD = \d+/, 'COVER_HOLD = 30').replace(/DISPLAY_WAIT = \d+/, 'DISPLAY_WAIT = 600');
class Widget {
    constructor(props = {}) { Object.assign(this, props); this.children = []; this.effect = props.effect; }
    add_child(child) { this.children.push(child); }
    destroy_all_children() { this.children = []; }
    get_children() { return this.children; }
    set_size() {}
    get_effect() { return this.effect; }
    destroy() {}
}
class Effect { set(values) { Object.assign(this, values); } }
const factory = new Function('Clutter', 'Gio', 'GLib', 'Shell', 'St', 'LoginManager', 'Main',
    'HandoffGate', 'revealFrame', 'log', 'logError', 'global',
    source + '; return {registerSession, begin, reveal, finish, settled, whenSettled, setModulesIdle, whenFirstFrame, trace};');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function scenario({sessionActive, proxyDelay = 0}) {
    const messages = [];
    const handlers = {};
    const session = {
        Active: sessionActive,
        connect: (name, callback) => { handlers[name] = callback; return 7; },
        disconnect: () => { delete handlers['g-properties-changed']; },
    };
    let registered = 0;
    class Gate { constructor(o) { this.o = o; } hold() {} release() { if (this.done) return false; this.done = true; registered++; this.o.register(); return true; } }
    const stage = {
        add_child() {}, set_child_above_sibling() {}, disconnect() {},
        connect: (name, callback) => { handlers[name] = callback; return 3; },
    };
    const module = factory(
        {FixedLayout: class {}, AnimationMode: {}},
        {File: {new_for_path: () => ({get_uri: () => 'file:///login.jpg'})}},
        {get_monotonic_time: () => Math.round(performance.now() * 1000), getenv: () => null,
            file_test: () => true, FileTest: {IS_REGULAR: 1}},
        {BlurEffect: Effect},
        {Widget, ThemeContext: {get_for_stage: () => ({scale_factor: 1, connect: () => 1, disconnect() {}})},
            Settings: {get: () => ({enable_animations: false})}, Adjustment: class {}},
        {getLoginManager: () => ({getCurrentSessionProxy: async () => {
            if (proxyDelay)
                await sleep(proxyDelay);
            return session;
        }})},
        {sessionMode: {isGreeter: false, currentMode: 'user'},
            layoutManager: {monitors: [{x: 0, y: 0, width: 4, height: 4}], connect: () => 1, disconnect() {}}},
        Gate, (p, o) => ({radius: 0, brightness: 1, opacity: 0}),
        message => messages.push(message), error => { throw error; },
        {stage, screen_width: 4, screen_height: 4});
    return {module, messages, handlers, session, registered: () => registered};
}
const marks = messages => messages.map(m => m.replace(/^Nodalix login transition: /, '').replace(/ \(\+\d+ ms\)$/, ''));
(async () => {
    // GDM keeps the greeter on screen for a while: nothing is revealed until the session is active.
    let {module, messages, handlers, session} = await scenario({sessionActive: false});
    module.setModulesIdle(() => Promise.resolve());
    module.registerSession(() => {});
    module.begin();
    handlers['after-paint']();            // painted, but the screen still shows the greeter
    module.reveal();
    module.finish();                      // the shell's own start-up is over
    await sleep(250);
    assert.ok(!marks(messages).includes('revealing the desktop'), messages.join('\n'));
    assert.ok(module.settled());          // the modules may load under the cover meanwhile
    session.Active = true;
    handlers['g-properties-changed']();   // GDM switches the screen to the session
    await sleep(200);
    const order = marks(messages);
    assert.ok(order.indexOf('session active') < order.indexOf('revealing the desktop'), order.join('\n'));
    assert.ok(order.includes('desktop revealed'), order.join('\n'));

    // A screen that is already ours is revealed as soon as the cover has been shown for a moment.
    ({module, messages, handlers} = await scenario({sessionActive: true}));
    module.setModulesIdle(() => Promise.resolve());
    module.registerSession(() => {});
    module.begin();
    handlers['after-paint']();
    module.reveal();
    module.finish();
    await sleep(250);
    assert.ok(marks(messages).includes('desktop revealed'), messages.join('\n'));

    // If GDM never hands the screen over, the desktop is revealed anyway after a bound.
    ({module, messages, handlers} = await scenario({sessionActive: false}));
    module.registerSession(() => {});
    module.begin();
    handlers['after-paint']();
    module.reveal();
    module.finish();
    await sleep(900);
    assert.ok(marks(messages).includes('desktop revealed'), messages.join('\n'));

    // The desktop modules wait for the cover's first frame (plus a moment to reach the screen).
    ({module, messages, handlers} = await scenario({sessionActive: true}));
    let loadable = false;
    assert.strictEqual(await Promise.race([module.whenFirstFrame(), Promise.resolve('idle')]), undefined);   // no transition: no wait
    module.registerSession(() => {});
    module.begin();
    module.whenFirstFrame().then(() => { loadable = true; });
    await sleep(30);
    assert.strictEqual(loadable, false);
    handlers['after-paint']();
    await sleep(20);
    assert.strictEqual(loadable, false);        // the frame is still on its way to the screen
    await sleep(200);
    assert.strictEqual(loadable, true, messages.join('\n'));
    assert.ok(marks(messages).includes('desktop modules may load: cover painted'), messages.join('\n'));
    assert.ok(marks(messages).some(m => m.startsWith('cover state: ')), messages.join('\n'));   // never throws, even on odd stubs
    module.trace('module test ready in 5 ms');
    assert.ok(marks(messages).includes('module test ready in 5 ms'), messages.join('\n'));

    // Without a first frame the modules are released by the bound, never held forever.
    ({module, messages, handlers} = await scenario({sessionActive: true}));
    module.registerSession(() => {});
    module.begin();
    let bounded = false;
    module.whenFirstFrame().then(() => { bounded = true; });
    await sleep(1700);
    assert.strictEqual(bounded, true, messages.join('\n'));
    assert.ok(marks(messages).includes('desktop modules may load: no first frame yet'), messages.join('\n'));

    // A session lookup that finishes after the transition is over connects nothing.
    ({module, messages, handlers} = await scenario({sessionActive: false, proxyDelay: 1000}));
    module.registerSession(() => {});
    module.begin();
    handlers['after-paint']();
    module.reveal();
    module.finish();
    await sleep(1300);
    assert.ok(marks(messages).includes('desktop revealed'), messages.join('\n'));
    assert.strictEqual(handlers['g-properties-changed'], undefined, 'a watcher leaked after settle()');
    assert.ok(!marks(messages).includes('session not active yet'), messages.join('\n'));
    process.exit(0);    // scenarios that never finish leave their safety timers pending
})().catch(error => { console.error(error); process.exit(1); });
"""
        subprocess.run(["node", "-e", script, str(self.overlay / "transition.js")], check=True)

    def test_video_wallpaper_waits_for_the_transition_and_does_not_block_other_modules(self):
        script = r"""
const assert = require('assert');
const fs = require('fs');
const source = fs.readFileSync(process.argv[1], 'utf8');
const body = source.slice(source.indexOf('const AFTER_TRANSITION')).replaceAll('export ', '');
const factory = new Function('Gio', 'St', 'Transition', 'ExtensionState', 'ExtensionType',
    'loadExtensionMetadata', 'logError', 'PHONE_LINK_ID', 'bundledIds', 'packagedIds',
    body + '; return {DesktopModules, AFTER_TRANSITION, records};');
const hanabi = 'hanabi-extension@jeffshee.github.io';
(async () => {
    let settled = false;
    const waiters = [];
    const Transition = {
        settled: () => settled,
        whenSettled: () => new Promise(resolve => waiters.push(resolve)),
        setModulesIdle: () => {},
        whenFirstFrame: () => Promise.resolve(),
        trace: () => {},
    };
    const {DesktopModules, AFTER_TRANSITION, records} = factory(
        {}, {}, Transition, {}, {}, () => {}, error => { throw error; }, 'phone', new Set(), new Set());
    assert.ok(AFTER_TRANSITION.has(hanabi));
    const enabled = [];
    let scheduled = 0;
    const manager = Object.create(DesktopModules.prototype);
    Object.assign(manager, {
        stopped: false, requested: true, idleWaiters: [],
        selectedModules: () => ['dash', hanabi, 'blur'], supportsSession: () => true,
        disable: () => { throw new Error('nothing should be disabled'); },
        enable: async uuid => { enabled.push(uuid); records.set(uuid, {}); },
        schedule: () => { scheduled++; },
    });
    await manager.reconcile();
    assert.deepStrictEqual(enabled, ['dash', 'blur']);   // the wallpaper never blocks the rest
    await manager.reconcile();
    assert.strictEqual(waiters.length, 1);               // and the wait is registered once
    settled = true;
    waiters.splice(0).forEach(resolve => resolve());
    await Promise.resolve();
    assert.strictEqual(scheduled, 1);                    // settling schedules a new pass
    await manager.reconcile();
    assert.deepStrictEqual(enabled, ['dash', 'blur', hanabi]);
    // whenIdle resolves at once when nothing is pending, and later otherwise.
    manager.running = false; manager.requested = false;
    await manager.whenIdle();
    manager.running = true;
    let done = false;
    manager.whenIdle().then(() => { done = true; });
    await Promise.resolve();
    assert.strictEqual(done, false);
    manager.idleWaiters.splice(0).forEach(resolve => resolve());
    await Promise.resolve();
    assert.strictEqual(done, true);
    // Nothing is enabled before the login cover has had its first frame.
    let frame = false;
    const frameWaiters = [];
    Transition.whenFirstFrame = () => frame ? Promise.resolve() : new Promise(resolve => frameWaiters.push(resolve));
    enabled.length = 0;
    records.clear();
    Object.assign(manager, {requested: true});
    const pass = manager.reconcile();
    await new Promise(resolve => setTimeout(resolve, 20));
    assert.deepStrictEqual(enabled, []);
    assert.strictEqual(manager.requested, true);       // a pending pass keeps whenIdle waiting
    frame = true;
    frameWaiters.splice(0).forEach(resolve => resolve());
    await pass;
    assert.deepStrictEqual(enabled, ['dash', hanabi, 'blur']);
    // A shell that shuts down while waiting for the frame enables nothing.
    frame = false;
    enabled.length = 0;
    records.clear();
    Object.assign(manager, {requested: true, stopped: false});
    const late = manager.reconcile();
    manager.stopped = true;
    frameWaiters.splice(0).forEach(resolve => resolve());
    await late;
    assert.deepStrictEqual(enabled, []);
})().catch(error => { console.error(error); process.exit(1); });
"""
        subprocess.run(["node", "-e", script, str(self.overlay / "modules.js")], check=True)

    def test_unrecognized_upstream_version_fails_without_writing(self):
        fixture(self.source, "52.0")
        before = (self.source / "js/ui/main.js").read_text()
        with self.assertRaisesRegex(ValueError, "pinned"):
            MODULE.prepare(self.source, self.overlay)
        self.assertEqual((self.source / "js/ui/main.js").read_text(), before)
        self.assertFalse((self.source / "js/nodalix").exists())

    def test_upstream_anchor_drift_fails_without_writing(self):
        path = self.source / "js/ui/extensionSystem.js"
        path.write_text("changed upstream code\n")
        with self.assertRaisesRegex(ValueError, "anchor differs"):
            MODULE.prepare(self.source, self.overlay)
        self.assertFalse((self.source / "js/nodalix").exists())

    def test_missing_or_duplicate_transition_anchor_fails_without_writing(self):
        layout = self.source / "js/ui/layout.js"
        main = self.source / "js/ui/main.js"
        original = {layout: layout.read_text(), main: main.read_text()}
        cases = ((layout, MODULE.LAYOUT_IMPORT), (layout, MODULE.LAYOUT_FIRST_FRAME),
                 (layout, MODULE.LAYOUT_SESSION_ANIMATION), (layout, MODULE.LAYOUT_COMPLETE),
                 (main, MODULE.MAIN_REGISTER))
        for path, anchor in cases:
            for text in (original[path].replace(anchor, ""), original[path] + anchor):
                path.write_text(text)
                with self.assertRaisesRegex(ValueError, "anchor"):
                    MODULE.prepare(self.source, self.overlay)
                self.assertFalse((self.source / "js/nodalix").exists())
                self.assertEqual(main.read_text(), text if path == main else original[main])
                self.assertEqual(layout.read_text(), text if path == layout else original[layout])
            path.write_text(original[path])

    def test_overlay_cannot_be_applied_twice(self):
        MODULE.prepare(self.source, self.overlay)
        with self.assertRaises(ValueError):
            MODULE.prepare(self.source, self.overlay)

    def test_version_coupled_to_nodalix_release(self):
        version = (ROOT / "VERSION").read_text().strip()
        pkgbuild = (ROOT / "packaging/nodalix-gnome-shell/PKGBUILD").read_text()
        self.assertIn(f"pkgver={version}\n", pkgbuild)
        self.assertIn("conflicts=('gnome-shell' 'nodalix-shell-preview')", pkgbuild)
        self.assertNotIn("replaces=(", pkgbuild)
        components = (ROOT / "release/components.json").read_text()
        self.assertNotIn('"nodalix-shell-preview"', components)
        shell = next(row for row in json.loads(components)['components'] if row['id'] == 'gnome-shell')
        self.assertEqual(shell['package'], 'nodalix-gnome-shell')
        self.assertEqual(shell['version'], version)
        self.assertTrue(shell['required'])
        self.assertEqual(shell['restart'], 'system')
        self.assertIn('tools/build-nodalix-shell.sh', (ROOT / 'tools/build-packages.sh').read_text())
        self.assertIn('nodalix-gnome-shell', (ROOT / 'iso/installer/packages.txt').read_text().splitlines())

    def test_release_and_legacy_preview_builders_prepare_the_same_modules(self):
        root = self.source / 'repository'
        (root / 'tools').mkdir(parents=True)
        recipe = root / 'packaging/nodalix-gnome-shell'
        recipe.mkdir(parents=True)
        version = (ROOT / 'VERSION').read_text()
        (root / 'VERSION').write_text(version)
        shutil.copy2(ROOT / 'packaging/nodalix-gnome-shell/PKGBUILD', recipe / 'PKGBUILD')
        for name in [
            'build-nodalix-shell.sh',
            'build-nodalix-shell-preview.sh',
            'prepare-nodalix-shell.py',
            'patch-nowplaying-multimonitor.py',
        ]:
            shutil.copy2(ROOT / 'tools' / name, root / 'tools' / name)
        overlay = root / 'gnome/nodalix-shell'
        overlay.mkdir(parents=True)
        for name in MODULE.RESOURCES:
            shutil.copy2(self.overlay / name, overlay / name)
        (overlay / 'modules.json').write_text(json.dumps({'bundled': ['test@nodalix'], 'desktopPackages': []}))
        module = root / 'gnome/extensions/test@nodalix'
        module.mkdir(parents=True)
        (module / 'extension.js').write_text('// source fixture')
        for script, package in [('build-nodalix-shell.sh', 'nodalix-gnome-shell'),
                                ('build-nodalix-shell-preview.sh', 'nodalix-shell-preview')]:
            subprocess.run(['bash', str(root / 'tools' / script), '--prepare-only'], check=True,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            prepared = root / 'dist' / (package + '-source')
            text = (prepared / 'PKGBUILD').read_text()
            self.assertIn('pkgname=' + package, text)
            self.assertNotIn('AUTO_OVERLAY_SHA256', text)
            self.assertEqual('epoch=1' in text, package == 'nodalix-shell-preview')
            archive = subprocess.check_output(
                ['tar', '--zstd', '-tf', str(prepared / 'nodalix-shell-overlays.tar.zst')], text=True)
            self.assertIn('gnome/extensions/test@nodalix/extension.js', archive)



if __name__ == "__main__":
    unittest.main()
