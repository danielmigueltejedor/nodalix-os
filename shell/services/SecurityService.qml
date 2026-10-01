pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    property bool loading: false
    property string errorMessage: ""

    property string firewallProvider: ""
    property bool firewallInstalled: false
    property bool firewallActive: false
    property bool firewallEnabled: false

    property bool uefi: false
    property bool secureBootEnabled: false

    property bool packageSignaturesRequired: false
    property string pacmanSigLevel: ""
    property string localFileSigLevel: ""

    property bool keyringInstalled: false
    property string keyringVersion: ""

    property int foreignPackageCount: 0
    property int flatpakBroadCount: 0
    property var flatpakBroadApps: []
    property int privilegedServiceCount: 0
    property int servicesWithoutHardeningCount: 0

    function flatpakReasonText(reasons) {
        const values = Array.isArray(reasons) ? reasons : []
        const labels = []
        if (values.indexOf("host-filesystem") >= 0)
            labels.push(I18n.tr("System files"))
        if (values.indexOf("home-filesystem") >= 0)
            labels.push(I18n.tr("Home folder"))
        if (values.indexOf("all-devices") >= 0)
            labels.push(I18n.tr("All devices"))
        if (values.indexOf("session-bus") >= 0)
            labels.push(I18n.tr("Session bus"))
        if (values.indexOf("system-bus") >= 0)
            labels.push(I18n.tr("System bus"))
        return labels.join(" · ")
    }

    function refresh() {
        if (_probe.running) return
        loading = true
        errorMessage = ""
        _probe.running = true
    }

    property Process _probe: Process {
        command: ["/usr/bin/nodalix-security-status"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const data = JSON.parse(text || "{}")
                    const firewall = data.firewall || {}
                    root.firewallProvider = String(firewall.provider || "")
                    root.firewallInstalled = firewall.installed === true
                    root.firewallActive = firewall.active === true
                    root.firewallEnabled = firewall.enabled === true

                    const secureBoot = data.secure_boot || {}
                    root.uefi = secureBoot.uefi === true
                    root.secureBootEnabled = secureBoot.enabled === true

                    const signatures = data.pacman_signatures || {}
                    root.packageSignaturesRequired = signatures.required === true
                    root.pacmanSigLevel = (signatures.siglevels || []).join(" · ")
                    root.localFileSigLevel = String(signatures.local_file_siglevel || "")

                    const keyring = data.keyring || {}
                    root.keyringInstalled = keyring.installed === true
                    root.keyringVersion = String(keyring.version || "")

                    root.foreignPackageCount = Number((data.foreign_packages || {}).count || 0)
                    const flatpak = data.flatpak || {}
                    root.flatpakBroadCount = Number(flatpak.broad_count || 0)
                    root.flatpakBroadApps = Array.isArray(flatpak.broad_apps) ? flatpak.broad_apps : []

                    const services = data.nodalix_services || {}
                    root.privilegedServiceCount = Number(services.privileged_count || 0)
                    root.servicesWithoutHardeningCount = Number(services.without_hardening_count || 0)
                } catch (error) {
                    root.errorMessage = I18n.tr("Security status could not be read")
                }
            }
        }
        onExited: function(code) {
            root.loading = false
            if (code !== 0)
                root.errorMessage = I18n.tr("Security status could not be read")
        }
    }

    Component.onCompleted: refresh()
}
