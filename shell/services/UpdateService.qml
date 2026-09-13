pragma Singleton
import QtQuick
import Quickshell.Io

// Unified Nodalix, Arch, application and firmware updater. A password is kept
// only until sudo validates it, then discarded. Every privileged action is a
// fixed argument list; no UI text is interpolated into a command.
QtObject {
    id: root

    property bool systemChecking: false
    property bool nodalixChecking: false
    readonly property bool checking: systemChecking || nodalixChecking
    property bool running: false
    property bool changingChannel: false
    property bool passwordRequested: false
    property bool authenticating: false
    property bool passwordError: false
    property string pendingAction: ""
    property string _password: ""

    property int systemUpdates: 0
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
    property string channel: "beta"
    property string repository: ""
    property string releaseUrl: ""

    // Compatibility with the 0.1 settings view while it migrates to the
    // dedicated Nodalix OS update page.
    readonly property string nodalixCurrent: installedVersion || "—"
    readonly property string nodalixAvailable: latestVersion || "—"
    readonly property string nodalixNotes: releaseNotes
    readonly property string nodalixChannel: channel
    readonly property bool nodalixUpdateAvailable: updateAvailable
    readonly property bool nodalixShellUpdate: _componentWillUpdate("shell")
    readonly property bool nodalixAppsUpdate: _componentWillUpdate("phone-link")
    readonly property bool nodalixThemesUpdate: _componentWillUpdate("shell")
    readonly property bool nodalixChannelError: state === "error_recoverable"

    readonly property bool autoNodalix: SettingsService.get("updates.auto.nodalix", false)
    readonly property bool autoSystem: SettingsService.get("updates.auto.system", false)
    readonly property bool autoApps: SettingsService.get("updates.auto.apps", false)
    readonly property bool autoFirmware: SettingsService.get("updates.auto.firmware", false)

    function _componentWillUpdate(id) {
        for (const component of components)
            if (String(component.id || "") === id) return Boolean(component.will_update)
        return false
    }

    function check() {
        if (!systemChecking && !_systemCheck.running) {
            systemChecking = true
            _systemCheck.running = true
        }
        if (!nodalixChecking && !_nodalixCheck.running) {
            nodalixChecking = true
            state = "checking"
            errorMessage = ""
            _nodalixCheck.running = true
        }
    }

    function request(action) {
        if (running || authenticating) return
        passwordError = false
        errorMessage = ""
        if (action === "aur" || action === "flatpak" || action === "apps") {
            _runUnprivileged(action)
            return
        }
        if (action.indexOf("nodalix-") === 0) action = "nodalix"
        if (["nodalix", "system", "firmware", "all"].indexOf(action) < 0) {
            statusText = I18n.tr("This update action is not available")
            return
        }
        pendingAction = action
        passwordRequested = true
    }

    function setAutomatic(kind, enabled) {
        if (kind === "apps") {
            _runUnprivileged("auto-apps-" + (enabled ? "on" : "off"))
            return
        }
        pendingAction = "auto-" + kind + "-" + (enabled ? "on" : "off")
        passwordError = false
        passwordRequested = true
    }

    function setChannel(value) {
        if (value !== "stable" && value !== "beta") return
        pendingAction = "channel-" + value
        passwordError = false
        passwordRequested = true
    }

    function cancelPassword() {
        passwordRequested = false
        authenticating = false
        passwordError = false
        pendingAction = ""
        _password = ""
    }

    function submitPassword(password) {
        if (!password || authenticating || running || pendingAction === "") return
        _password = password
        passwordError = false
        authenticating = true
        running = true
        statusText = I18n.tr("Authorizing…")
        _authenticate.running = true
    }

    function _runAuthorized(action) {
        const enable = action.endsWith("-on")
        let command = []
        if (action === "nodalix")
            command = ["sudo", "-n", "/usr/bin/nodalix-updater", "update"]
        else if (action === "system")
            command = ["sudo", "-n", "pacman", "-Syu", "--noconfirm"]
        else if (action === "firmware")
            command = ["sudo", "-n", "sh", "-c", "fwupdmgr refresh --force && fwupdmgr update -y"]
        else if (action === "all")
            command = ["sudo", "-n", "/usr/bin/nodalix-system-update", "privileged-all"]
        else if (action.indexOf("channel-") === 0)
            command = ["sudo", "-n", "/usr/bin/nodalix-updater", "channel", action.substring(8), "--json"]
        else if (action.indexOf("auto-system-") === 0)
            command = ["sudo", "-n", "systemctl", enable ? "enable" : "disable", "--now", "nodalix-update-system.timer"]
        else if (action.indexOf("auto-firmware-") === 0)
            command = ["sudo", "-n", "systemctl", enable ? "enable" : "disable", "--now", "nodalix-update-firmware.timer"]
        else if (action.indexOf("auto-nodalix-") === 0)
            command = ["sudo", "-n", "systemctl", enable ? "enable" : "disable", "--now", "nodalix-update-nodalix.timer"]
        else {
            running = false
            pendingAction = ""
            statusText = I18n.tr("This update action is not available")
            return
        }
        statusText = I18n.tr("Updating in background…")
        _authorized.command = command
        _authorized.running = true
    }

    function _runUnprivileged(action) {
        let command = []
        if (action === "aur") command = ["sh", "-c", "command -v yay >/dev/null && yay -Sua --noconfirm"]
        else if (action === "flatpak") command = ["flatpak", "update", "-y", "--noninteractive"]
        else if (action === "apps") command = ["/usr/bin/nodalix-system-update", "user-apps"]
        else if (action.indexOf("auto-apps-") === 0)
            command = ["systemctl", "--user", action.endsWith("-on") ? "enable" : "disable", "--now", "nodalix-update-apps.timer"]
        if (command.length === 0) return
        pendingAction = action
        running = true
        statusText = I18n.tr("Updating in background…")
        _unprivileged.command = command
        _unprivileged.running = true
    }

    function _finish(code, action) {
        running = false
        authenticating = false
        _password = ""
        if (code === 0) {
            statusText = I18n.tr("Update completed")
            const automatic = /^auto-(nodalix|system|apps|firmware)-(on|off)$/.exec(action)
            if (automatic)
                SettingsService.set("updates.auto." + automatic[1], automatic[2] === "on")
            if (action.indexOf("channel-") === 0) channel = action.substring(8)
            pendingAction = ""
            check()
        } else {
            state = "error_recoverable"
            statusText = errorMessage || I18n.tr("The update finished with errors")
        }
    }

    function _acceptInfo(data) {
        installedVersion = String(data.current_version || data.version || "")
        latestVersion = String(data.latest_version || installedVersion)
        updateAvailable = Boolean(data.update_available)
        components = Array.isArray(data.components) ? data.components : []
        channel = String(data.channel || channel)
        repository = String(data.repository || "")
        releaseUrl = String(data.html_url || "")
        releaseNotes = String(data.notes || "")
        shellRestartRequired = Boolean(data.shell_restart_required)
        rebootRequired = Boolean(data.reboot_required)
        state = String(data.status || (updateAvailable ? "update_available" : "up_to_date"))
        statusText = updateAvailable ? I18n.tr("Nodalix update available") : I18n.tr("Nodalix is up to date")
        lastChecked = new Date().toLocaleString(Qt.locale(I18n.localeName))
    }

    property Process _authenticate: Process {
        command: ["sudo", "-S", "-p", "", "-v"]
        stdinEnabled: true
        stdout: StdioCollector {}
        stderr: StdioCollector {}
        onStarted: {
            write(root._password + "\n")
            root._password = ""
        }
        onExited: function(code) {
            root.authenticating = false
            if (code === 0) {
                root.passwordRequested = false
                root.passwordError = false
                root._runAuthorized(root.pendingAction)
            } else {
                root.running = false
                root.passwordError = true
                root.statusText = I18n.tr("Incorrect password")
            }
        }
    }

    property Process _authorized: Process {
        stdout: StdioCollector {}
        stderr: StdioCollector {
            onStreamFinished: if (text.trim() !== "") root.errorMessage = text.trim()
        }
        onExited: function(code) {
            const completed = root.pendingAction
            if (code === 0 && completed === "all") {
                root.pendingAction = "apps"
                root.running = false
                root._runUnprivileged("apps")
            } else {
                root._finish(code, completed)
            }
        }
    }

    property Process _unprivileged: Process {
        stdout: StdioCollector {}
        stderr: StdioCollector {
            onStreamFinished: if (text.trim() !== "") root.errorMessage = text.trim()
        }
        onExited: function(code) { root._finish(code, root.pendingAction) }
    }

    property Process _systemCheck: Process {
        command: ["sh", "-c", "sys=$(timeout 15s checkupdates 2>/dev/null | wc -l); aur=$(timeout 15s yay -Qua 2>/dev/null | wc -l); flat=$(timeout 15s flatpak remote-ls --updates --columns=application 2>/dev/null | sed '/^[[:space:]]*$/d' | wc -l); printf '%s|%s|%s\\n' \"$sys\" \"$aur\" \"$flat\""]
        stdout: StdioCollector {
            onStreamFinished: {
                const fields = text.trim().split("|")
                root.systemUpdates = parseInt(fields[0]) || 0
                root.aurUpdates = parseInt(fields[1]) || 0
                root.flatpakUpdates = parseInt(fields[2]) || 0
                root.lastChecked = new Date().toLocaleString(Qt.locale(I18n.localeName))
            }
        }
        onExited: root.systemChecking = false
    }

    property Process _nodalixCheck: Process {
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
            root.nodalixChecking = false
            if (code !== 0) {
                root.state = "error_recoverable"
                root.statusText = root.errorMessage || I18n.tr("Could not check for updates")
            }
        }
    }

    Component.onCompleted: check()
}
