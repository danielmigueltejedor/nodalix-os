#!/usr/bin/env python3
"""Apply Nodalix multi-monitor fixes to Now Playing Card 1.0.14 after unzip.

Preserves upstream attribution: the ZIP license and metadata stay intact.
This script only extends BuiltinMediaHider, shares one MPRIS source across
hosts, and attaches independent panel actors to secondary monitors.
"""
from __future__ import annotations

import argparse
import os
import pathlib
import sys
import tempfile

NOWPLAYING_UUID = "nowplaying@epogonii.github.io"

SHARED_SOURCE = '''
let _sharedMprisSource = null;
let _sharedMprisRefs = 0;

function acquireSharedMprisSource() {
    if (!_sharedMprisSource)
        _sharedMprisSource = new MprisSource();
    _sharedMprisRefs += 1;
    return _sharedMprisSource;
}

function releaseSharedMprisSource() {
    _sharedMprisRefs = Math.max(0, _sharedMprisRefs - 1);
    if (_sharedMprisRefs === 0 && _sharedMprisSource) {
        _sharedMprisSource.destroy();
        _sharedMprisSource = null;
    }
}
'''

HIDER_CLASS = '''
// The shell shows its own media controls in the notification list, which would
// duplicate the card. Hide them on the primary date menu and every secondary
// multi-monitors-bar date menu. Fully undone when the extension stops.
class BuiltinMediaHider {
    constructor() {
        this._hidden = false;
        this._views = [];
    }

    _allMessageViews() {
        const views = [];
        const collect = panel => {
            const view = panel?.statusArea?.dateMenu?._messageList?._messageView;
            if (view && !views.includes(view))
                views.push(view);
        };
        collect(Main.panel);
        for (const panel of (Main.mmPanel ?? []))
            collect(panel);
        return views;
    }

    hide() {
        const views = this._allMessageViews();
        let changed = false;
        for (const view of views) {
            if (view?._mediaSource && view._playerToMessage) {
                view._mediaSource.disconnectObject(view);
                this._dropMessages(view);
                changed = true;
            }
        }
        const lists = [];
        const collectList = panel => {
            const list = panel?.statusArea?.dateMenu?._messageList;
            if (list?._mediaSection?.get_parent())
                lists.push(list);
        };
        collectList(Main.panel);
        for (const panel of (Main.mmPanel ?? []))
            collectList(panel);
        for (const list of lists) {
            const section = list._mediaSection;
            section.get_parent().remove_child(section);
            changed = true;
        }
        // Merge so a second hide() after remove_child does not forget lists
        // that restore() still needs to reinsert.
        const merge = (current, next) => {
            const out = [...(current ?? [])];
            for (const item of next) {
                if (!out.includes(item))
                    out.push(item);
            }
            return out;
        };
        this._views = merge(this._views, views);
        this._lists = merge(this._lists, lists);
        this._hidden = changed || this._hidden || this._lists.length > 0;
    }

    restore() {
        if (!this._hidden)
            return;
        this._hidden = false;

        for (const view of this._views ?? this._allMessageViews()) {
            try {
                if (view?._setupMpris) {
                    this._dropMessages(view);
                    view._setupMpris();
                }
            } catch (error) {
                logError(error, 'Now Playing media view restore failed');
            }
        }
        for (const list of this._lists ?? []) {
            try {
                const section = list?._mediaSection;
                if (section && !section.get_parent() && list?._sectionList)
                    list._sectionList.insert_child_at_index(section, 0);
            } catch (error) {
                logError(error, 'Now Playing media section restore failed');
            }
        }
        this._views = [];
        this._lists = [];
    }

    _dropMessages(view) {
        [...view._playerToMessage.keys()].forEach(
            player => view._removePlayer(player));
    }

    destroy() {
        this.restore();
    }
}
'''


def resolve_extension_js(path: pathlib.Path) -> pathlib.Path:
    """Accept only the unpacked Now Playing Card extension.js path.

    The CLI argument is untrusted packaging input: reject traversal, symlinks
    that escape the module directory, and any file that is not extension.js.
    """
    if path is None or str(path) == "":
        raise SystemExit("missing Now Playing extension.js path")
    raw = os.fspath(path)
    if "\x00" in raw:
        raise SystemExit("invalid Now Playing extension.js path")
    candidate = pathlib.Path(raw).expanduser()
    if candidate.name != "extension.js":
        raise SystemExit("refusing to patch a file that is not named extension.js")
    if candidate.is_symlink():
        raise SystemExit("refusing to patch through a symbolic link")
    try:
        resolved = candidate.resolve(strict=True)
    except OSError as error:
        raise SystemExit(f"Now Playing extension.js is not readable: {error}") from error
    if resolved.name != "extension.js":
        raise SystemExit("refusing to patch a file that is not named extension.js")
    if resolved.parent.name != NOWPLAYING_UUID:
        raise SystemExit(f"refusing to patch extension.js outside {NOWPLAYING_UUID}/")
    if not resolved.is_file():
        raise SystemExit("Now Playing extension.js must be a regular file")
    # Keep the final path inside the verified module directory.
    if resolved.parent != candidate.parent.resolve():
        raise SystemExit("refusing to patch outside the Now Playing module directory")
    return resolved


def _read_text(path: pathlib.Path) -> str:
    flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        fd = os.open(path, flags)
    except OSError as error:
        raise SystemExit(f"Now Playing extension.js is not readable: {error}") from error
    with os.fdopen(fd, "r", encoding="utf-8") as handle:
        return handle.read()


def _write_text(path: pathlib.Path, text: str) -> None:
    directory = path.parent
    mode = path.stat().st_mode
    fd, temporary_name = tempfile.mkstemp(
        prefix=".extension.js.",
        suffix=".tmp",
        dir=directory,
    )
    temporary = pathlib.Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise


def _replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"unexpected Now Playing sources: missing {label}")
    return text.replace(old, new, 1)


def _require_anchor(text: str, marker: str, label: str) -> int:
    try:
        return text.index(marker)
    except ValueError as error:
        raise SystemExit(f"unexpected Now Playing sources: missing {label}") from error


def patch(path: pathlib.Path) -> None:
    target = resolve_extension_js(path)
    text = _read_text(target)
    if "acquireSharedMprisSource" in text and "_allMessageViews" in text:
        return

    if "class MprisSource extends Signals.EventEmitter" not in text:
        raise SystemExit("unexpected Now Playing sources: MprisSource missing")
    if "this._source = new MprisSource();" not in text:
        raise SystemExit("unexpected Now Playing sources: MediaModel source missing")

    text = _replace_once(
        text,
        "class MprisSource extends Signals.EventEmitter {",
        SHARED_SOURCE + "\nclass MprisSource extends Signals.EventEmitter {",
        "MprisSource class",
    )

    text = _replace_once(
        text,
        "class MediaModel {\n    constructor(settings, {keepStackVisible, divided, closeMenu}) {\n        this._settings = settings;\n        this._players = new Set();\n        this._shown = new Set();\n\n        this._analyzer = new SpectrumAnalyzer();\n        this.equalizer = new EqualizerIcon();\n        this.stack = new CardStack(closeMenu, keepStackVisible, divided);\n\n        this._source = new MprisSource();\n",
        "class MediaModel {\n    constructor(settings, {keepStackVisible, divided, closeMenu, sharedSource = null}) {\n        this._settings = settings;\n        this._players = new Set();\n        this._shown = new Set();\n        this._ownsSource = sharedSource === null;\n\n        this._analyzer = new SpectrumAnalyzer();\n        this.equalizer = new EqualizerIcon();\n        this.stack = new CardStack(closeMenu, keepStackVisible, divided);\n\n        this._source = sharedSource ?? new MprisSource();\n",
        "MediaModel constructor",
    )

    text = _replace_once(
        text,
        "    destroy() {\n        this._stopListening();\n        this._source?.destroy();\n        this._source = null;\n",
        "    destroy() {\n        this._stopListening();\n        if (this._ownsSource) {\n            this._source?.destroy();\n        } else if (this._source) {\n            releaseSharedMprisSource();\n        }\n        this._source = null;\n",
        "MediaModel.destroy",
    )

    text = _replace_once(
        text,
        "        this._model = new MediaModel(settings, {\n            keepStackVisible: true,\n            divided: true,\n            closeMenu: () => this.menu.close(),\n        });\n",
        "        this._model = new MediaModel(settings, {\n            keepStackVisible: true,\n            divided: true,\n            closeMenu: () => this.menu.close(),\n            sharedSource: acquireSharedMprisSource(),\n        });\n",
        "NowPlayingButton MediaModel construction",
    )

    start = _require_anchor(
        text,
        "// The shell shows its own media controls in the notification list",
        "BuiltinMediaHider comment",
    )
    end = _require_anchor(text, "export default class NowPlayingExtension extends Extension", "extension export")
    text = text[:start] + HIDER_CLASS + "\n" + text[end:]

    old_enable_tail = """        this._hider = new BuiltinMediaHider();
        this._build();
        this._syncBuiltinMedia();
    }

    disable() {
        this._settings?.disconnectObject(this);
        this._settings = null;
        this._destroyHost();
        this._hider?.destroy();
        this._hider = null;
    }
"""
    new_enable_tail = """        this._hider = new BuiltinMediaHider();
        this._secondaryHosts = [];
        this._build();
        this._syncBuiltinMedia();
        this._monitorsChangedId = Main.layoutManager.connect(
            'monitors-changed', () => this._scheduleSecondaryRefresh());
        this._scheduleSecondaryRefresh();
    }

    disable() {
        this._settings?.disconnectObject(this);
        this._settings = null;
        if (this._secondaryRefreshId) {
            GLib.source_remove(this._secondaryRefreshId);
            this._secondaryRefreshId = 0;
        }
        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }
        this._destroySecondaryHosts();
        this._destroyHost();
        this._hider?.destroy();
        this._hider = null;
    }

    _scheduleSecondaryRefresh() {
        if (this._secondaryRefreshId)
            GLib.source_remove(this._secondaryRefreshId);
        this._secondaryRefreshId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._secondaryRefreshId = 0;
            this._attachSecondaryHosts();
            this._syncBuiltinMedia();
            return GLib.SOURCE_REMOVE;
        });
    }

    _destroySecondaryHosts() {
        for (const host of this._secondaryHosts ?? []) {
            try {
                host.destroy();
            } catch (error) {
                logError(error, 'Now Playing secondary host cleanup failed');
            }
        }
        this._secondaryHosts = [];
    }

    _attachSecondaryHosts() {
        if (readSetting(this._settings, 'location') === 'quick-settings') {
            this._destroySecondaryHosts();
            return;
        }
        const panels = Main.mmPanel ?? [];
        const box = readSetting(this._settings, 'panel-box');
        const index = readSetting(this._settings, 'panel-index');
        const keep = [];
        for (const panel of panels) {
            if (!panel || panel === Main.panel)
                continue;
            const role = `${this.uuid}::${panel.monitorIndex ?? keep.length}`;
            if (panel.statusArea?.[role]) {
                keep.push(panel.statusArea[role]);
                continue;
            }
            const host = new NowPlayingButton(this._settings,
                () => this.openPreferences());
            const targetBox = box === 'left' ? panel._leftBox
                : box === 'right' ? panel._rightBox
                : panel._centerBox;
            if (typeof panel.addToStatusArea === 'function') {
                panel.addToStatusArea(role, host, index, box);
            } else if (typeof panel._addToPanelBox === 'function' && targetBox) {
                panel._addToPanelBox(role, host, index, targetBox);
            } else {
                host.destroy();
                continue;
            }
            keep.push(host);
        }
        for (const host of this._secondaryHosts ?? []) {
            if (!keep.includes(host)) {
                try {
                    host.destroy();
                } catch (error) {
                    logError(error, 'Now Playing stale secondary host cleanup failed');
                }
            }
        }
        this._secondaryHosts = keep;
    }
"""
    text = _replace_once(text, old_enable_tail, new_enable_tail, "enable/disable block")

    old_rebuild = """    _rebuild() {
        this._destroyHost();
        this._build();
    }
}
"""
    new_rebuild = """    _rebuild() {
        this._destroySecondaryHosts();
        this._destroyHost();
        this._build();
        this._scheduleSecondaryRefresh();
    }
}
"""
    text = _replace_once(text, old_rebuild, new_rebuild, "rebuild block")

    # hide() must be re-runnable after secondary date menus are rebuilt.
    text = _replace_once(
        text,
        """    _syncBuiltinMedia() {
        if (readSetting(this._settings, 'hide-builtin-media'))
            this._hider?.hide();
        else
            this._hider?.restore();
    }
""",
        """    _syncBuiltinMedia() {
        if (readSetting(this._settings, 'hide-builtin-media')) {
            this._hider._hidden = false;
            this._hider?.hide();
        } else {
            this._hider?.restore();
        }
    }
""",
        "_syncBuiltinMedia",
    )

    _write_text(target, text)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("extension_js", type=pathlib.Path)
    args = parser.parse_args()
    target = resolve_extension_js(args.extension_js)
    patch(target)
    print(f"patched {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
