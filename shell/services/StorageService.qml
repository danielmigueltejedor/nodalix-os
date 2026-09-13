pragma Singleton
import QtQuick
import Quickshell.Io

QtObject {
    id: root
    property bool loading: false
    property bool appsLoading: false
    property real totalBytes: 0
    property real usedBytes: 0
    property var categories: []
    property var apps: []
    property bool passwordRequested: false
    property bool authenticating: false
    property bool passwordError: false
    property string pendingPackage: ""
    property string _password: ""
    property string statusText: ""
    readonly property var protectedPackages: ["base", "linux", "linux-firmware", "systemd", "pacman", "sudo", "greetd", "hyprland", "quickshell", "networkmanager", "fish"]

    function formatBytes(n) {
        n = Number(n) || 0
        const units = ["B", "KB", "MB", "GB", "TB"]
        let i = 0
        while (n >= 1024 && i < units.length - 1) { n /= 1024; i++ }
        return (i < 2 ? Math.round(n) : n.toFixed(1)) + " " + units[i]
    }
    function refresh() { if (!_scan.running) { loading = true; _scan.running = true } }
    function loadApps() { if (!_apps.running) { appsLoading = true; _apps.running = true } }
    function requestUninstall(kind, id) {
        if (!id || authenticating) return
        if (kind === "pacman" && protectedPackages.indexOf(id) >= 0) {
            statusText = I18n.tr("This system component is protected")
            return
        }
        if (kind === "flatpak") {
            pendingPackage = id; statusText = I18n.tr("Uninstalling…")
            _flatpak.command = ["flatpak", "uninstall", "-y", "--noninteractive", id]; _flatpak.running = true
        } else {
            pendingPackage = id; passwordError = false; passwordRequested = true
        }
    }
    function cancelPassword() { passwordRequested = false; pendingPackage = ""; passwordError = false; _password = "" }
    function submitPassword(password) {
        if (!password || !/^[A-Za-z0-9@._+:-]+$/.test(pendingPackage)) return
        _password = password; authenticating = true; passwordError = false
        _remove.command = ["sudo", "-S", "-p", "", "pacman", "-Rns", "--noconfirm", pendingPackage]
        _remove.running = true
    }

    property Process _scan: Process {
        command: ["sh", "-c",
            "set -- $(df -B1 --output=size,used / | tail -1); printf 'disk|%s|%s\\n' \"$1\" \"$2\"; " +
            "for pair in 'Documents|DOCUMENTS' 'Downloads|DOWNLOAD' 'Pictures|PICTURES' 'Videos|VIDEOS' 'Music|MUSIC'; do " +
            "n=${pair%%|*}; k=${pair#*|}; p=$(xdg-user-dir \"$k\" 2>/dev/null); s=$(du -sb \"$p\" 2>/dev/null | cut -f1); printf '%s|%s\\n' \"$n\" \"${s:-0}\"; done; " +
            "a=$(du -sb /usr /opt \"$HOME/.local/share/flatpak\" /var/lib/flatpak 2>/dev/null | awk '{s+=$1} END{print s+0}'); printf 'Applications|%s\\n' \"$a\""]
        stdout: StdioCollector { onStreamFinished: {
            const out = []; const lines = text.trim().split("\n")
            for (let i = 0; i < lines.length; i++) {
                const f = lines[i].split("|")
                if (f[0] === "disk") { root.totalBytes = Number(f[1]); root.usedBytes = Number(f[2]) }
                else if (f.length >= 2) out.push({ key: f[0], bytes: Number(f[1]) || 0 })
            }
            let known = 0; for (let j = 0; j < out.length; j++) known += out[j].bytes
            out.push({ key: "System and other", bytes: Math.max(0, root.usedBytes - known) })
            root.categories = out; root.loading = false
        } }
        onExited: root.loading = false
    }
    property Process _apps: Process {
        command: ["sh", Paths.configDir + "/scripts/storage/list-apps.sh"]
        stdout: StdioCollector { onStreamFinished: {
            const out = []; for (const ln of text.trim().split("\n")) {
                const f = ln.split("|"); if (f.length < 3) continue
                out.push({
                    kind: f[0], id: f[1], size: f[2],
                    name: f.length > 3 && f[3] !== "" ? f[3] : f[1],
                    icon: f.length > 4 && f[4] !== "" ? f[4] : "application-x-executable"
                })
            }
            out.sort((a,b) => (a.name || "").localeCompare(b.name || "")); root.apps = out; root.appsLoading = false
        } }
        onExited: root.appsLoading = false
    }
    property Process _remove: Process {
        stdinEnabled: true
        onStarted: { write(root._password + "\n"); root._password = ""; root.statusText = I18n.tr("Uninstalling…") }
        onExited: function(code) {
            root.authenticating = false; root._password = ""
            if (code === 0) { root.passwordRequested = false; root.statusText = I18n.tr("Application uninstalled"); root.loadApps(); root.refresh() }
            else { root.passwordError = true; root.statusText = I18n.tr("Uninstall failed") }
        }
    }
    property Process _flatpak: Process { onExited: function(code) { root.statusText = code === 0 ? I18n.tr("Application uninstalled") : I18n.tr("Uninstall failed"); root.loadApps(); root.refresh() } }
    Component.onCompleted: refresh()
}
