#!/usr/bin/env python3
"""Measure how long the screen stalls when the style switches between light and dark.

A small window draws every frame; the longest gap between two frames after the switch is the
stall. The switch is repeated with the shell's modules turned off one by one (and all together),
so the table shows which one costs the most. Everything it changes is put back at the end.

    python3 tools/style-switch-probe.py            # all modules, one at a time
    python3 tools/style-switch-probe.py --quick    # only: everything on, everything off
    python3 tools/style-switch-probe.py --blur     # Blur my Shell's parts, one at a time
"""
import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gio, GLib, Gtk  # noqa: E402

SETTLE_MS = 1500
WATCH_MS = 2500


BLUR_SCHEMA = "org.gnome.shell.extensions.blur-my-shell"
BLUR_PARTS = ["panel", "dash-to-dock", "overview", "appfolder", "applications", "window-list",
              "coverflow-alt-tab", "screenshot", "popup"]


def blur_parts():
    """The parts of Blur my Shell that are on now, as (name, Gio.Settings)."""
    source = Gio.SettingsSchemaSource.get_default()
    found = []
    for part in BLUR_PARTS:
        schema = source.lookup(f"{BLUR_SCHEMA}.{part}", True)
        if schema and schema.has_key("blur"):
            settings = Gio.Settings.new_full(schema, None, None)
            if settings.get_boolean("blur"):
                found.append((part, settings))
    return found


def main(quick, blur):
    interface = Gio.Settings.new("org.gnome.desktop.interface")
    shell = Gio.Settings.new("org.gnome.shell")
    modules = shell.get_strv("enabled-extensions")
    saved = (interface.get_string("color-scheme"), shell.get_strv("disabled-extensions"))

    parts = blur_parts() if blur else []
    configs = [("everything on", [], [])]
    if blur:
        configs.append(("all of Blur my Shell off", [], [p for p, _ in parts]))
        configs += [(f"blur: {p} off", [], [p]) for p, _ in parts]
    elif modules:
        configs.append(("all modules off", modules, []))
        if not quick:
            configs += [(f"{m} off", [m], []) for m in modules]

    gaps = []
    last = [0]

    def tick(_widget, clock):
        now = clock.get_frame_time()
        if last[0]:
            gaps.append((now, (now - last[0]) / 1000))
        last[0] = now
        return GLib.SOURCE_CONTINUE

    Gtk.init()
    window = Gtk.Window(title="style-switch-probe", default_width=280, default_height=80)
    window.set_child(Gtk.Label(label="measuring…"))
    window.add_tick_callback(tick)
    window.present()
    loop = GLib.MainLoop()
    results = []

    def stall_after(start):
        return max((gap for t, gap in gaps if t >= start), default=0.0)

    def steps():
        for name, off, blur_off in configs:
            shell.set_strv("disabled-extensions", sorted(set(saved[1]) | set(off)))
            for part, settings in parts:
                settings.set_boolean("blur", part not in blur_off)
            yield SETTLE_MS
            worst = 0.0
            for scheme in ("prefer-light" if saved[0] == "prefer-dark" else "prefer-dark", saved[0]):
                start = last[0]
                interface.set_string("color-scheme", scheme)
                yield WATCH_MS
                worst = max(worst, stall_after(start))
            results.append((name, worst))
            print(f"{worst:7.0f} ms  {name}", flush=True)
        loop.quit()

    runner = steps()

    def advance():
        try:
            delay = next(runner)
        except StopIteration:
            return GLib.SOURCE_REMOVE
        GLib.timeout_add(delay, advance)
        return GLib.SOURCE_REMOVE

    GLib.idle_add(advance)
    try:
        loop.run()
    finally:
        interface.set_string("color-scheme", saved[0])
        shell.set_strv("disabled-extensions", saved[1])
        for _part, settings in parts:
            settings.set_boolean("blur", True)
        Gio.Settings.sync()
    if results:
        base = results[0][1]
        print(f"\nLongest stall with everything on: {base:.0f} ms (60 Hz frame = 17 ms).")
        best = min(results[1:], key=lambda r: r[1], default=None)
        if best and best[1] < base * 0.6:
            print(f"Biggest drop: {best[0]} ({best[1]:.0f} ms).")


if __name__ == "__main__":
    main("--quick" in sys.argv, "--blur" in sys.argv)
