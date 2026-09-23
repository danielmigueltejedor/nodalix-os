pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    readonly property string mode: {
        const saved = SettingsService.get("windows.layoutMode", "desktop")
        return saved === "tabs" ? "desktop" : saved
    }
    property bool busy: false
    property string _pendingMode: ""
    property string statusText: ""

    readonly property var modes: [
        { id: "desktop", name: I18n.tr("Windows"), description: I18n.tr("Free windows with a dock and visible controls") },
        { id: "tiling", name: I18n.tr("Mosaic"), description: I18n.tr("Automatic Hyprland tiling") },
        { id: "scrolling", name: I18n.tr("Flow"), description: I18n.tr("Super + Left / Right moves between columns") }
    ]

    function setMode(next) {
        if (busy || ["desktop", "tiling", "scrolling"].indexOf(next) < 0) return
        _pendingMode = next
        statusText = I18n.tr("Applying window layout…")
        _apply.command = ["python3", Paths.configDir + "/scripts/nodalix-layout-mode.py", next]
        _apply.running = true
    }

    Component.onCompleted: {
        if (SettingsService.get("windows.layoutMode", "desktop") === "tabs") setMode("desktop")
    }

    function cycle() {
        const order = ["desktop", "tiling", "scrolling"]
        const at = Math.max(0, order.indexOf(mode))
        setMode(order[(at + 1) % order.length])
    }

    property Process _apply: Process {
        running: false
        onRunningChanged: root.busy = running
        onExited: function(code) {
            if (code === 0) SettingsService.set("windows.layoutMode", root._pendingMode)
            root.statusText = code === 0
                ? I18n.tr("Window layout applied")
                : I18n.tr("The window layout could not be applied")
            _clear.restart()
        }
    }
    property Timer _clear: Timer { interval: 4000; onTriggered: root.statusText = "" }
}
