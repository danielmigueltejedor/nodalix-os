pragma Singleton
import QtQuick
import Quickshell.Io

// Background updater. The password is sent once through stdin to sudo and is
// immediately discarded; fixed commands only, never user-provided shell text.
QtObject {
    id: root
    property bool checking: false
    property bool running: false
    property int systemUpdates: 0
    property int aurUpdates: 0
    property int flatpakUpdates: 0
    property string lastChecked: ""
    property string statusText: ""
    property bool passwordRequested: false
    property bool authenticating: false
    property bool passwordError: false
    property string pendingAction: ""
    property string _password: ""

    readonly property bool autoSystem: SettingsService.get("updates.auto.system", false)
    readonly property bool autoApps: SettingsService.get("updates.auto.apps", false)
    readonly property bool autoFirmware: SettingsService.get("updates.auto.firmware", false)

    function check() { if (!_check.running) { checking = true; _check.running = true } }

    function request(action) {
        if (running || authenticating) return
        passwordError = false
        if (action === "aur" || action === "flatpak" || action === "apps") {
            _runUnprivileged(action)
        } else {
            pendingAction = action
            passwordRequested = true
        }
    }

    function setAutomatic(kind, enabled) {
        const action = "auto-" + kind + "-" + (enabled ? "on" : "off")
        if (kind === "apps") _runUnprivileged(action)
        else { pendingAction = action; passwordError = false; passwordRequested = true }
    }

    function cancelPassword() {
        passwordRequested = false; pendingAction = ""; passwordError = false; _password = ""
    }

    function submitPassword(password) {
        if (!password || authenticating || running) return
        authenticating = true; running = true; passwordError = false; _password = password
        _privileged.command = ["sh", "-lc", _privilegedCommand(pendingAction)]
        _privileged.running = true
    }

    function _privilegedCommand(action) {
        if (action === "system") return "sudo -S -p '' pacman -Syu --noconfirm"
        if (action === "firmware") return "sudo -S -p '' sh -c 'fwupdmgr refresh --force && fwupdmgr update -y'"
        if (action === "all") return "sudo -S -p '' sh -c 'pacman -Syu --noconfirm && fwupdmgr refresh --force && fwupdmgr update -y' && yay -Sua --noconfirm && flatpak update -y --noninteractive"
        const m = /^auto-(system|firmware)-(on|off)$/.exec(action)
        if (m) return "sudo -S -p '' systemctl " + (m[2] === "on" ? "enable --now " : "disable --now ") + "nodalix-update-" + m[1] + ".timer"
        return "false"
    }

    function _runUnprivileged(action) {
        let cmd = "false"
        if (action === "aur") cmd = "yay -Sua --noconfirm"
        else if (action === "flatpak") cmd = "flatpak update -y --noninteractive"
        else if (action === "apps") cmd = "yay -Sua --noconfirm && flatpak update -y --noninteractive"
        else if (action.indexOf("auto-apps-") === 0)
            cmd = "systemctl --user " + (action.endsWith("-on") ? "enable --now " : "disable --now ") + "nodalix-update-apps.timer"
        pendingAction = action; running = true; statusText = I18n.tr("Updating in background…")
        _unprivileged.command = ["sh", "-lc", cmd]; _unprivileged.running = true
    }

    function _finish(code, action) {
        running = false; authenticating = false; _password = ""
        if (code === 0) {
            passwordRequested = false; passwordError = false; statusText = I18n.tr("Update completed")
            const m = /^auto-(system|apps|firmware)-(on|off)$/.exec(action)
            if (m) SettingsService.set("updates.auto." + m[1], m[2] === "on")
            check()
        } else {
            passwordError = action !== "aur" && action !== "flatpak" && action !== "apps"
            statusText = I18n.tr("The update finished with errors")
        }
        pendingAction = ""
    }

    property Process _privileged: Process {
        stdinEnabled: true
        stdout: StdioCollector {}
        stderr: StdioCollector {}
        onStarted: { write(root._password + "\n"); root._password = ""; root.statusText = I18n.tr("Updating in background…") }
        onExited: function(code) { root._finish(code, root.pendingAction) }
    }
    property Process _unprivileged: Process {
        stdout: StdioCollector {}
        stderr: StdioCollector {}
        onExited: function(code) { root._finish(code, root.pendingAction) }
    }
    property Process _check: Process {
        command: ["sh", "-c", "sys=$(timeout 15s checkupdates 2>/dev/null | wc -l); aur=$(timeout 15s yay -Qua 2>/dev/null | wc -l); flat=$(timeout 15s flatpak remote-ls --updates --columns=application 2>/dev/null | sed '/^[[:space:]]*$/d' | wc -l); printf '%s|%s|%s\\n' \"$sys\" \"$aur\" \"$flat\""]
        stdout: StdioCollector { onStreamFinished: {
            const f = text.trim().split("|"); root.systemUpdates = parseInt(f[0]) || 0; root.aurUpdates = parseInt(f[1]) || 0; root.flatpakUpdates = parseInt(f[2]) || 0
            root.lastChecked = new Date().toLocaleTimeString(Qt.locale(I18n.localeName), "HH:mm"); root.checking = false
        } }
        onExited: root.checking = false
    }
    Component.onCompleted: check()
}
