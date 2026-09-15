pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    property bool busy: false
    property bool failed: false
    property string statusText: ""

    function apply(fontFamily) {
        if (busy || fontFamily === "") return
        busy = true
        failed = false
        statusText = I18n.tr("Applying system font…")
        // Packaged shell scripts need not have the executable bit set.
        _apply.command = ["/bin/sh", Paths.configDir + "/scripts/nodalix-set-font.sh", fontFamily]
        _deadline.running = true
        _apply.running = true
    }

    property Timer _deadline: Timer {
        interval: 15000
        running: false
        onTriggered: {
            if (!root.busy) return
            root._apply.running = false
            root.busy = false
            root.failed = true
            root.statusText = I18n.tr("System font could not be applied")
        }
    }

    property Process _apply: Process {
        running: false
        onExited: (code, status) => {
            root._deadline.running = false
            if (!root.busy) return
            root.busy = false
            root.failed = code !== 0
            root.statusText = code === 0
                ? I18n.tr("Font applied · reopen applications to finish")
                : I18n.tr("System font could not be applied")
        }
    }
}
