pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    property bool loading: false
    property bool busy: false
    property bool configured: false
    property bool active: false
    property bool enabled: false
    property string exposedName: "Nodalix"
    property bool autoReconnect: true
    property bool notifications: true
    property bool calls: true
    property bool copyCodes: false
    property string statusText: ""

    function refresh() {
        if (_status.running) return
        loading = true
        _status.running = true
    }

    function setEnabled(value) { _run(["enable", value ? "true" : "false"], I18n.tr("Changing Phone Link state…")) }
    function setName(value) { _run(["set", "name", value], I18n.tr("Updating visible name…")) }
    function setOption(name, value) { _run(["set", name, value ? "true" : "false"], I18n.tr("Updating Phone Link settings…")) }
    function restart() { _run(["action", "restart"], I18n.tr("Restarting Phone Link…")) }
    function syncContacts() { _run(["action", "contacts"], I18n.tr("Synchronizing contacts…")) }
    function openApp() { _run(["action", "open"], I18n.tr("Opening Phone Link…")) }

    function _run(args, message) {
        if (busy) return
        statusText = message
        _action.command = ["/usr/bin/nodalix-phone-link-settings"].concat(args)
        _action.running = true
    }

    property Process _status: Process {
        command: ["/usr/bin/nodalix-phone-link-settings", "status"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const data = JSON.parse(text)
                    root.configured = data.configured === true
                    root.active = data.active === true
                    root.enabled = data.enabled === true
                    root.exposedName = data.name || "Nodalix"
                    root.autoReconnect = data.auto_reconnect !== false
                    root.notifications = data.notifications !== false
                    root.calls = data.calls !== false
                    root.copyCodes = data.copy_codes === true
                } catch (e) {
                    root.statusText = I18n.tr("Phone Link status is unavailable")
                }
            }
        }
        onExited: root.loading = false
    }

    property Process _action: Process {
        running: false
        onRunningChanged: root.busy = running
        onExited: function(code) {
            root.statusText = code === 0 ? I18n.tr("Phone Link settings applied") : I18n.tr("The Phone Link setting could not be applied")
            root.refresh()
            _clear.restart()
        }
    }
    property Timer _clear: Timer { interval: 5000; onTriggered: root.statusText = "" }

    Component.onCompleted: refresh()
}
