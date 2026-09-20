pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    readonly property string mode: SettingsService.get("windows.layoutMode", "desktop")
    property bool busy: false
    property string _pendingMode: ""
    property string statusText: ""

    readonly property var modes: [
        { id: "desktop", icon: "󰍹", name: I18n.tr("Desktop"), description: I18n.tr("Free windows with a dock and visible controls") },
        { id: "tiling", icon: "󰕰", name: I18n.tr("Mosaic"), description: I18n.tr("Automatic Hyprland tiling") },
        { id: "scrolling", icon: "󰁔", name: I18n.tr("Flow"), description: I18n.tr("Super + Left / Right moves between columns") },
        { id: "tabs", icon: "󰓩", name: I18n.tr("Tabs"), description: I18n.tr("One visible window per native tab group") }
    ]

    function setMode(next) {
        if (busy || ["desktop", "tiling", "scrolling", "tabs"].indexOf(next) < 0) return
        _pendingMode = next
        statusText = I18n.tr("Applying window layout…")
        _apply.command = ["/usr/bin/nodalix-layout-mode", next]
        _apply.running = true
    }

    function cycle() {
        const order = ["desktop", "tiling", "scrolling", "tabs"]
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
