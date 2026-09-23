pragma Singleton
import QtQuick
import Quickshell.Io

QtObject {
    id: root
    property var data: ({})
    property bool busy: false
    property string message: ""
    function load() { if (!_read.running) _read.running = true }
    function perform(action, value, confirmed) {
        if (busy) return
        busy = true; message = I18n.tr("Working…")
        let command = ["pkexec", "/usr/bin/nodalix-recovery", action]
        if (value) command.push(value)
        if (confirmed) command.push("--confirm")
        _action.command = command; _action.running = true
    }
    function advanced() { if (!_advanced.running) _advanced.running = true }
    property Process _read: Process {
        command: ["/usr/bin/nodalix-recovery", "status"]
        stdout: StdioCollector { onStreamFinished: { try { root.data = JSON.parse(text) } catch (_) {} } }
    }
    property Process _action: Process {
        stdout: StdioCollector { onStreamFinished: {
            try {
                const result = JSON.parse(text)
                if (result.error) root.message = result.error
                else { root.data = result; root.message = I18n.tr("Recovery points updated") }
            } catch (_) { root.message = I18n.tr("Recovery could not complete. Check administrator authorization.") }
        } }
        onExited: function(code) {
            root.busy = false
            if (code !== 0 && root.message === I18n.tr("Working…")) root.message = I18n.tr("Recovery could not complete. Check administrator authorization.")
        }
    }
    property Process _advanced: Process { command: ["timeshift-launcher"] }
    Component.onCompleted: load()
}
