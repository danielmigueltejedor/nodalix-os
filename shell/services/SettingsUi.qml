pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import "."

// Open/close state for the Settings window (centered modal). Opens on the
// focused monitor. Keyboard + focus restore are handled by the window's
// HyprlandFocusGrab (see Settings.qml).
QtObject {
    id: root

    property bool   open:       false
    property string screenName: ""
    property string category:   "appearance-group"   // selected settings group/page

    function _focusedName() { return Hyprland.focusedMonitor?.name ?? "" }

    function show() {
        screenName = _focusedName()
        open = true
        OverlayManager.open("settings", screenName)
    }
    function hide() {
        OverlayManager.close("settings", screenName)
        open = false
    }
    function toggle() {
        const screen = _focusedName()
        const action = OverlayManager.toggle("settings", screen)
        if (action === "closed") {
            open = false
            return
        }
        screenName = screen
        open = true
    }

    property Connections _overlayConn: Connections {
        target: OverlayManager
        function onClosed(overlayId, screen) {
            if (overlayId === "settings" && (screen === root.screenName || root.screenName === ""))
                root.open = false
        }
        function onOpened(overlayId, screen) {
            if (overlayId === "settings") {
                root.screenName = screen
                root.open = true
            }
        }
        function onRaised(overlayId, screen) {
            if (overlayId === "settings") {
                root.screenName = screen
                root.open = true
            }
        }
    }
}
