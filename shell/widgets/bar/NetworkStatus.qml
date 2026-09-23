import QtQuick
import QtQuick.Layouts
import Quickshell.Networking
import Quickshell.Io
import "../../theme"
import "../../services"

Item {
    id: root

    property var barScreen: null

    implicitWidth:  _icon.implicitWidth
    implicitHeight: _icon.implicitHeight

    // Reactive scan: bindings re-evaluate whenever the device list, a network's
    // `connected`, or its `signalStrength` changes (QML subscribes to every
    // notifiable property touched while evaluating). The connected wifi network
    // is a WifiNetwork, which carries `signalStrength` (the device does not).
    readonly property var _devices: Networking.devices?.values ?? []

    readonly property var connectedWifi: {
        for (const d of _devices) {
            if (!d || d.type !== DeviceType.Wifi) continue
            const nets = d.networks?.values ?? []
            for (const n of nets) if (n && n.connected) return n
        }
        return null
    }
    readonly property bool _hasEthernet: {
        for (const d of _devices) if (d && d.type === DeviceType.Ethernet && d.connected) return true
        return false
    }
    property bool _nmEthernet: false
    property Process _ethernetProbe: Process {
        command: ["sh", "-c", "nmcli -t -f TYPE,STATE device status 2>/dev/null | awk -F: '$1 == \"ethernet\" && $2 == \"connected\" { found=1 } END { print found+0 }'"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: root._nmEthernet = text.trim() === "1"
        }
    }
    Timer {
        // Quickshell networking is reactive; this is only a slow fallback for
        // drivers that do not emit their wired state correctly.
        interval: 15000
        repeat: true
        running: true
        onTriggered: {
            root._ethernetProbe.running = false
            root._ethernetProbe.running = true
        }
    }
    Component.onCompleted: _ethernetProbe.running = true

    // Wired networking wins when both are connected: never show a Wi-Fi icon
    // for a desktop that is actively using Ethernet.
    readonly property bool isEthernet: _hasEthernet || _nmEthernet
    readonly property bool isWifi:     !isEthernet && connectedWifi !== null

    ShellIcon {
        id: _icon
        anchors.centerIn: parent
        iconName: {
            if (isEthernet) return "network-wired-symbolic"
            if (!isWifi) return "network-wireless-offline-symbolic"
            // signalStrength is a 0..1 fraction.
            const s = root.connectedWifi ? root.connectedWifi.signalStrength : 0
            if (s >= 0.8) return "nm-signal-100-symbolic"
            if (s >= 0.6) return "nm-signal-75-symbolic"
            if (s >= 0.4) return "nm-signal-50-symbolic"
            if (s >= 0.2) return "nm-signal-25-symbolic"
            return "nm-signal-0-symbolic"
        }
        color: (isWifi || isEthernet) ? ThemeManager.onSurface : ThemeManager.onSurfaceVariant
        opacity: (isWifi || isEthernet) ? 1.0 : 0.5
    }

    HoverHandler {
        onHoveredChanged: {
            const pos = root.mapToItem(null, root.width / 2, 0)
            if (hovered) {
                PopoutService.open("network", pos.x, root.barScreen)
                PopoutService.widgetHovered = true
            } else {
                PopoutService.widgetHovered = false
            }
        }
    }
}
