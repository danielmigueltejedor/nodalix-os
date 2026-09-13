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
        _apply.command = [Paths.configDir + "/scripts/nodalix-set-font.sh", fontFamily]
        _apply.running = true
    }

    property Process _apply: Process {
        running: false
        onExited: (code, status) => {
            root.busy = false
            root.failed = code !== 0
            root.statusText = code === 0
                ? I18n.tr("Font applied · reopen applications to finish")
                : I18n.tr("System font could not be applied")
        }
    }
}
