pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import "."

// Open/close state for the app launcher (Win11-style start menu).
// Opens on the currently focused monitor. MainWindow draws the launcher blob +
// content for the screen whose name matches `screenName` while `open`.
QtObject {
    id: root

    property bool   open:       false
    property string screenName: ""   // monitor the launcher should appear on
    property string query:      ""   // live search text

    function _focusedName() {
        const m = Hyprland.focusedMonitor
        return m?.name ?? ""
    }

    // Keyboard + automatic focus restore are handled by MainWindow's
    // HyprlandFocusGrab while the launcher is open.
    //
    // Super+Space: close if launcher is already the top overlay; otherwise
    // open or raise it above whatever is on that monitor.
    function show() {
        screenName = _focusedName()
        query = ""
        open = true
        OverlayManager.open("launcher", screenName)
    }
    function hide() {
        OverlayManager.close("launcher", screenName)
        open = false
        query = ""
    }
    function toggle() {
        const screen = _focusedName()
        const action = OverlayManager.toggle("launcher", screen)
        if (action === "closed") {
            open = false
            query = ""
            return
        }
        screenName = screen
        open = true
        if (action === "opened")
            query = ""
    }

    property Connections _overlayConn: Connections {
        target: OverlayManager
        function onClosed(overlayId, screen) {
            if (overlayId === "launcher" && (screen === root.screenName || root.screenName === "")) {
                root.open = false
                root.query = ""
            }
        }
        function onOpened(overlayId, screen) {
            if (overlayId === "launcher") {
                root.screenName = screen
                root.open = true
            }
        }
        function onRaised(overlayId, screen) {
            if (overlayId === "launcher") {
                root.screenName = screen
                root.open = true
            }
        }
    }
}
