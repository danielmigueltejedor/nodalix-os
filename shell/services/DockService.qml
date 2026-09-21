pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import "."

QtObject {
    id: root
    function address(win) { const value = win?.lastIpcObject?.address || win?.address || ""; return value.startsWith("0x") ? value : "0x" + value }
    function appFor(win) {
        const ipc = win.lastIpcObject || {}
        return AppService.byClass(ipc["class"] || win.wayland?.appId || ipc.initialClass || "")
    }
    readonly property var groups: {
        const result = []
        const byKey = {}
        for (const pinned of PinnedService.pinned) {
            const app = AppService.byKey(pinned)
            if (!app) continue
            const key = AppService.keyOf(app)
            if (byKey[key]) continue
            const group = {key: key, app: app, name: app.name, windows: [], pinned: true}
            byKey[key] = group; result.push(group)
        }
        for (const win of Hyprland.toplevels.values) {
            const app = appFor(win)
            const cls = win.lastIpcObject?.class || win.wayland?.appId || ""
            // Wait for the compositor metadata instead of showing a blank tile.
            if (!app && !cls) continue
            const key = app ? AppService.keyOf(app) : "window-class:" + cls.toLowerCase()
            if (!byKey[key]) {
                const group = {key: key, app: app, name: app?.name || cls, windows: [], pinned: false}
                byKey[key] = group; result.push(group)
            }
            byKey[key].windows.push(win)
        }
        return result
    }
    function run(action, win, monitorName) {
        if (!win) return
        Quickshell.execDetached(["python3", Paths.configDir + "/scripts/nodalix-window-action.py", action, address(win), "--monitor", monitorName || ""])
        _refresh.restart()
        _lateRefresh.restart()
    }
    function activate(group, monitorName) {
        if (!group.windows.length) { AppService.launch(group.app); return }
        const active = group.windows.indexOf(Hyprland.activeToplevel)
        const target = group.windows[active >= 0 ? (active + 1) % group.windows.length : 0]
        run("activate", target, monitorName)
    }
    property Timer _refresh: Timer { interval: 80; onTriggered: { Hyprland.refreshToplevels(); Hyprland.refreshMonitors() } }
    property Timer _lateRefresh: Timer { interval: 350; onTriggered: Hyprland.refreshToplevels() }
    property Connections _events: Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (["openwindow", "closewindow", "movewindow", "movewindowv2", "windowtitle", "windowtitlev2", "activewindowv2"].indexOf(event.name) >= 0) {
                root._refresh.restart(); root._lateRefresh.restart()
            }
        }
    }
    Component.onCompleted: _refresh.start()
}
