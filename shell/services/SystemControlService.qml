pragma Singleton
import QtQuick
import Quickshell.Io
import "."

// Central bridge for desktop-wide settings that do not yet have a native
// Quickshell API. Commands are fixed argument lists; editable values are passed
// as individual arguments and are never interpolated into shell source.
QtObject {
    id: root

    property bool loading: false
    property bool actionBusy: false
    property string statusText: ""

    property string timezone: "—"
    property bool ntpEnabled: false
    property bool ntpSynchronized: false

    property bool tailscaleDetected: false
    property bool tailscaleActive: false
    property string tailscaleIp: ""

    property string hostName: "—"
    property string osName: "—"
    property string kernel: "—"
    property string cpu: "—"
    property string memory: "—"
    property string gpu: "—"

    property string defaultBrowser: ""
    property string defaultFileManager: ""
    property string defaultMail: ""
    property string defaultText: ""
    property string defaultImage: ""
    property string defaultPdf: ""

    signal defaultsChanged()

    function normalizedGpuName(value) {
        let name = (value || "").trim()
        if (name === "") return "—"

        // PCI names contain vendor aliases, internal codenames, every model
        // sharing the device ID and a revision. Keep the useful product family
        // so the About page remains scannable on compact layouts.
        name = name.replace(/\s+\(rev [^)]+\)$/i, "")
        const product = name.match(/\[((?:Radeon|GeForce|Arc)\s+[^\]]+)\]/i)
        if (product) name = product[1]

        let vendor = ""
        if (/Advanced Micro Devices|\[AMD\/ATI\]|\bAMD\b/i.test(value)) vendor = "AMD"
        else if (/NVIDIA/i.test(value)) vendor = "NVIDIA"
        else if (/Intel/i.test(value)) vendor = "Intel"

        name = name
            .replace(/^Advanced Micro Devices, Inc\.\s*(?:\[AMD\/ATI\])?\s*/i, "")
            .replace(/^NVIDIA Corporation\s*/i, "")
            .replace(/^Intel Corporation\s*/i, "")
            .trim()

        if (name.includes("/")) name = name.split("/", 1)[0].trim() + " Series"
        if (vendor !== "" && !name.toLowerCase().startsWith(vendor.toLowerCase() + " "))
            name = vendor + " " + name
        return name || "—"
    }

    function refresh() {
        if (_probe.running) return
        loading = true
        _probe.running = true
    }

    function setNtp(enabled) {
        if (actionBusy) return
        statusText = I18n.tr("Applying date and time settings…")
        _action.command = ["timedatectl", "set-ntp", enabled ? "true" : "false"]
        _action.running = true
    }

    function setTimezone(value) {
        const zone = (value || "").trim()
        if (actionBusy || zone === "") return
        statusText = I18n.tr("Applying date and time settings…")
        _action.command = ["timedatectl", "set-timezone", zone]
        _action.running = true
    }

    function toggleTailscale() {
        if (actionBusy || !tailscaleDetected) return
        statusText = tailscaleActive ? I18n.tr("Disconnecting Tailscale…") : I18n.tr("Connecting Tailscale…")
        _action.command = ["tailscale", tailscaleActive ? "down" : "up"]
        _action.running = true
    }

    function setDefault(role, desktopId) {
        if (actionBusy || !desktopId) return
        const scripts = {
            browser: "xdg-settings set default-web-browser \"$1\" && xdg-mime default \"$1\" x-scheme-handler/http && xdg-mime default \"$1\" x-scheme-handler/https",
            files:   "xdg-mime default \"$1\" inode/directory",
            mail:    "xdg-mime default \"$1\" x-scheme-handler/mailto",
            text:    "xdg-mime default \"$1\" text/plain",
            image:   "for m in image/png image/jpeg image/webp image/gif; do xdg-mime default \"$1\" \"$m\" || exit 1; done",
            pdf:     "xdg-mime default \"$1\" application/pdf"
        }
        if (!scripts[role]) return
        statusText = I18n.tr("Changing default application…")
        _action.command = ["sh", "-c", scripts[role], "nodalix-default-app", desktopId]
        _action.running = true
    }

    property Process _probe: Process {
        command: ["sh", "-c",
            "export LC_ALL=C; " +
            "printf 'timezone=%s\\n' \"$(timedatectl show -p Timezone --value 2>/dev/null)\"; " +
            "printf 'ntp=%s\\n' \"$(timedatectl show -p NTP --value 2>/dev/null)\"; " +
            "printf 'ntpsync=%s\\n' \"$(timedatectl show -p NTPSynchronized --value 2>/dev/null)\"; " +
            "printf 'hostname=%s\\n' \"$(hostnamectl --static 2>/dev/null || hostname)\"; " +
            "printf 'os=%s\\n' \"$(. /etc/os-release 2>/dev/null; printf '%s' \"${PRETTY_NAME:-Linux}\")\"; " +
            "printf 'kernel=%s\\n' \"$(uname -r)\"; " +
            "printf 'cpu=%s\\n' \"$(awk -F: '/^model name[ \\t]*:/{gsub(/^[ \\t]+/,\"\",$2); print $2; exit}' /proc/cpuinfo 2>/dev/null)\"; " +
            "printf 'memory=%s\\n' \"$(free -h 2>/dev/null | awk '/Mem:/{print $2}')\"; " +
            "printf 'gpu=%s\\n' \"$(/usr/bin/lspci 2>/dev/null | awk -F': ' '/VGA compatible controller|3D controller|Display controller/{print $2; exit}')\"; " +
            "printf 'browser=%s\\n' \"$(xdg-settings get default-web-browser 2>/dev/null)\"; " +
            "printf 'files=%s\\n' \"$(xdg-mime query default inode/directory 2>/dev/null)\"; " +
            "printf 'mail=%s\\n' \"$(xdg-mime query default x-scheme-handler/mailto 2>/dev/null)\"; " +
            "printf 'text=%s\\n' \"$(xdg-mime query default text/plain 2>/dev/null)\"; " +
            "printf 'image=%s\\n' \"$(xdg-mime query default image/png 2>/dev/null)\"; " +
            "printf 'pdf=%s\\n' \"$(xdg-mime query default application/pdf 2>/dev/null)\"; " +
            "if command -v tailscale >/dev/null 2>&1; then " +
            "  state=$(tailscale status --json 2>/dev/null | jq -r '.BackendState // \"\"'); " +
            "  ip=$(tailscale ip -4 2>/dev/null | head -n1); " +
            "  printf 'tailscale=1\\ntailscaleActive=%s\\ntailscaleIp=%s\\n' \"$([ \"$state\" = Running ] && echo 1 || echo 0)\" \"$ip\"; " +
            "else printf 'tailscale=0\\ntailscaleActive=0\\ntailscaleIp=\\n'; fi"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const data = {}
                for (const line of text.split("\n")) {
                    const at = line.indexOf("=")
                    if (at > 0) data[line.substring(0, at)] = line.substring(at + 1)
                }
                root.timezone = data.timezone || "—"
                root.ntpEnabled = data.ntp === "yes"
                root.ntpSynchronized = data.ntpsync === "yes"
                root.hostName = data.hostname || "—"
                root.osName = data.os || "—"
                root.kernel = data.kernel || "—"
                root.cpu = data.cpu || "—"
                root.memory = data.memory || "—"
                root.gpu = root.normalizedGpuName(data.gpu)
                root.defaultBrowser = data.browser || ""
                root.defaultFileManager = data.files || ""
                root.defaultMail = data.mail || ""
                root.defaultText = data.text || ""
                root.defaultImage = data.image || ""
                root.defaultPdf = data.pdf || ""
                root.tailscaleDetected = data.tailscale === "1"
                root.tailscaleActive = data.tailscaleActive === "1"
                root.tailscaleIp = data.tailscaleIp || ""
                root.loading = false
                root.defaultsChanged()
            }
        }
        onExited: root.loading = false
    }

    property Process _action: Process {
        running: false
        stderr: StdioCollector {}
        onRunningChanged: root.actionBusy = running
        onExited: function(code) {
            root.statusText = code === 0 ? I18n.tr("Setting applied") : I18n.tr("The setting could not be applied")
            root.refresh()
            _statusClear.restart()
        }
    }

    property Timer _statusClear: Timer {
        interval: 5000
        onTriggered: root.statusText = ""
    }

    // Static information is refreshed when Settings opens and after an action.
    // Avoid a permanent background poll for values that almost never change.
    Component.onCompleted: refresh()
}
