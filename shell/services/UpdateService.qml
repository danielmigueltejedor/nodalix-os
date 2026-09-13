pragma Singleton
import QtQuick
import Quickshell.Io

// Structured bridge to the Nodalix updater. Privileged work is delegated to
// the Polkit-authorized backend; passwords never enter the QML process.
QtObject {
    id: root
    property bool checking: false
    property bool running: false
    property int systemUpdates: updateAvailable ? components.length : 0
    property int aurUpdates: 0
    property int flatpakUpdates: 0
    property string lastChecked: ""
    property string statusText: ""
    property string state: "idle"
    property string installedVersion: ""
    property string latestVersion: ""
    property bool updateAvailable: false
    property var components: []
    property string releaseNotes: ""
    property bool shellRestartRequired: false
    property bool rebootRequired: false
    property string errorMessage: ""

    // Compatibility for the current Settings view. Polkit replaces its old
    // password prompt; these properties keep that prompt hidden.
    readonly property bool passwordRequested: false
    readonly property bool authenticating: false
    readonly property bool passwordError: false
    readonly property bool autoSystem: SettingsService.get("updates.auto.system", false)
    readonly property bool autoApps: false
    readonly property bool autoFirmware: false

    function check() {
        if (checking || running) return
        checking = true
        state = "checking"
        errorMessage = ""
        _check.running = false
        _check.running = true
    }

    function request(action) {
        if (running || checking) return
        if (action !== "all" && action !== "system") {
            statusText = I18n.tr("This component is managed outside Nodalix Update")
            return
        }
        running = true
        state = "installing"
        errorMessage = ""
        statusText = I18n.tr("Installing Nodalix update…")
        _update.running = false
        _update.running = true
    }

    function setAutomatic(kind, enabled) {
        if (kind !== "system") return
        SettingsService.set("updates.auto.system", enabled)
        statusText = I18n.tr("Automatic update preference saved")
    }

    function cancelPassword() {}
    function submitPassword(password) {}

    function _acceptInfo(data) {
        installedVersion = String(data.current_version || data.version || "")
        latestVersion = String(data.latest_version || installedVersion)
        updateAvailable = Boolean(data.update_available)
        components = Array.isArray(data.components)
            ? data.components.filter(c => Boolean(c.will_update)) : []
        releaseNotes = String(data.notes || "")
        shellRestartRequired = Boolean(data.shell_restart_required)
        rebootRequired = Boolean(data.reboot_required)
        state = String(data.status || (updateAvailable ? "update_available" : "up_to_date"))
        statusText = updateAvailable
            ? I18n.tr("Nodalix update available")
            : I18n.tr("Nodalix is up to date")
        lastChecked = new Date().toLocaleString(Qt.locale(I18n.localeName))
    }

    property Process _check: Process {
        command: ["nodalix-updater", "check", "--json"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { root._acceptInfo(JSON.parse(text)) }
                catch (e) {
                    root.state = "error_recoverable"
                    root.errorMessage = I18n.tr("The updater returned invalid data")
                    root.statusText = root.errorMessage
                }
            }
        }
        stderr: StdioCollector {
            onStreamFinished: if (text.trim() !== "") root.errorMessage = text.trim()
        }
        onExited: function(code) {
            root.checking = false
            if (code !== 0) {
                root.state = "error_recoverable"
                root.statusText = root.errorMessage || I18n.tr("Could not check for updates")
            }
        }
    }

    property Process _update: Process {
        command: ["pkexec", "/usr/bin/nodalix-updater", "update"]
        stdout: StdioCollector {}
        stderr: StdioCollector {
            onStreamFinished: if (text.trim() !== "") root.errorMessage = text.trim()
        }
        onExited: function(code) {
            root.running = false
            if (code === 0) {
                root.statusText = I18n.tr("Update completed")
                root.check()
            } else {
                root.state = "error_recoverable"
                root.statusText = root.errorMessage || I18n.tr("The update finished with errors")
            }
        }
    }

    Component.onCompleted: check()
}
