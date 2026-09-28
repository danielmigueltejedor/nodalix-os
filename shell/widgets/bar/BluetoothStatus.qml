import QtQuick
import QtQuick.Layouts
import Quickshell.Bluetooth
import "../../theme"
import "../../services"

Item {
    id: root

    property var barScreen: null

    implicitWidth:  _row.implicitWidth
    implicitHeight: _row.implicitHeight

    readonly property var adapter: Bluetooth.defaultAdapter
    // Bar indicator represents adapter power only.
    // Treat Enabling as ON and Disabling as OFF so the glyph reacts
    // immediately when the user toggles Bluetooth.
    readonly property bool btEnabled:
        adapter !== null
        && (
            adapter.state === BluetoothAdapterState.Enabled
            || adapter.state === BluetoothAdapterState.Enabling
        )


    RowLayout {
        id: _row
        anchors.fill: parent
        spacing: 4

        ShellIcon {
            role: "bluetooth.device"
            // The top-bar indicator represents the Bluetooth adapter itself,
            // not whether an individual device is currently connected.
            state: root.btEnabled ? "connected" : "disabled"
            // Follow the shell foreground palette just like the other
            // status indicators. State is communicated by the glyph itself.
            color: root.btEnabled
                ? ThemeManager.onSurface
                : ThemeManager.onSurfaceVariant
            opacity: root.btEnabled ? 1.0 : 0.5
            Layout.alignment: Qt.AlignVCenter

            Behavior on color { ColorAnimation { duration: 120 } }
        }
    }

    HoverHandler {
        onHoveredChanged: {
            const pos = root.mapToItem(null, root.width / 2, 0)
            if (hovered) {
                PopoutService.open("bluetooth", pos.x, root.barScreen)
                PopoutService.widgetHovered = true
            } else {
                PopoutService.widgetHovered = false
            }
        }
    }
}
