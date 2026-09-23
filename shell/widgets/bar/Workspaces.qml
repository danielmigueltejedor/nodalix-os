import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import "../../theme"
import "../../services"

RowLayout {
    id: root

    required property var barScreen

    spacing: 4

    property var hyprMonitor: Hyprland.monitorFor(barScreen)

    readonly property bool _hideSpecial: SettingsService.get("bar.workspaces.hideSpecial", true)
    readonly property bool _numbers:     SettingsService.get("bar.workspaces.numbers", false)
    readonly property int  _base:        hyprMonitor ? hyprMonitor.id * 10 : 0

    // Hover anywhere over the dots → open the workspace overview popout,
    // anchored to the row's horizontal centre.
    HoverHandler {
        id: _wsHover
        onHoveredChanged: {
            if (hovered) {
                const c = root.mapToItem(null, root.width / 2, 0)
                PopoutService.open("workspaces", c.x, root.barScreen)
                PopoutService.widgetHovered = true
            } else if (PopoutService.currentName === "workspaces") {
                PopoutService.widgetHovered = false
            }
        }
    }

    Repeater {
        model: Hyprland.workspaces

        delegate: Item {
            id: workspaceItem
            required property var modelData  // HyprlandWorkspace

            // Quickshell assigns monitor after creating the delegate; a var
            // modelData.monitor binding does not reliably re-evaluate then.
            property string monitorName: ""
            Component.onCompleted: monitorName = modelData.monitor ? modelData.monitor.name : ""
            Connections {
                target: modelData
                function onMonitorChanged() {
                    workspaceItem.monitorName = modelData.monitor ? modelData.monitor.name : ""
                }
            }

            readonly property bool isActive:  modelData.active
            readonly property bool isFocused: modelData.focused
            readonly property bool isUrgent:  modelData.urgent

            // Quickshell can report id=-1 briefly for ordinary workspaces while
            // their IPC data settles. Special workspaces are identified by name.
            visible: root.barScreen && monitorName === root.barScreen.name
                     && (!root._hideSpecial || !String(modelData.name ?? "").startsWith("special:"))
            implicitWidth:  visible ? (root._numbers ? numText.implicitWidth + 8 : dot.implicitWidth) : 0
            implicitHeight: dot.implicitHeight
            Layout.alignment: Qt.AlignVCenter

            readonly property color _accent: isUrgent ? ThemeManager.error
                : isActive ? ThemeManager.primary : ThemeManager.onSurfaceVariant

            Rectangle {
                id: dot
                visible: !root._numbers
                anchors.centerIn: parent

                readonly property int dotSize: isActive ? 10 : 7
                implicitWidth:  dotSize
                implicitHeight: dotSize
                radius: dotSize / 2
                color:  parent._accent
                opacity: isActive ? 1.0 : 0.5

                Behavior on implicitWidth  { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
                Behavior on implicitHeight { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
                Behavior on color          { ColorAnimation   { duration: 120 } }
                Behavior on opacity        { NumberAnimation  { duration: 120 } }
            }

            Text {
                id: numText
                visible: root._numbers
                anchors.centerIn: parent
                text: modelData.id >= 0 ? (modelData.id - root._base)
                      : (String(modelData.name ?? "").startsWith("special:") ? "S" : modelData.name)
                color: parent._accent
                opacity: isActive ? 1.0 : 0.6
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeSm
                font.bold: isActive
            }

            MouseArea {
                anchors.fill: parent
                // Larger hit target than the dot
                anchors.margins: -6
                onClicked: modelData.activate()
                cursorShape: Qt.PointingHandCursor
            }
        }
    }
}
