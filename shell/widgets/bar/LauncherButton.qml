import QtQuick
import "../../theme"
import "../../services"

Rectangle {
    id: btn

    implicitWidth:  24
    implicitHeight: 24
    radius: ThemeManager.chipRadius / 2
    color: hovered ? ThemeManager.primaryContainer : "transparent"

    property bool hovered: false

    Behavior on color { ColorAnimation { duration: 100 } }

    ColloidIcon {
        anchors.centerIn: parent
        iconName: "application-menu-symbolic"
        group: "actions"
        fallbackGlyph: "󰀻"
        color: hovered ? ThemeManager.onPrimaryContainer : ThemeManager.onSurfaceVariant

        Behavior on color { ColorAnimation { duration: 100 } }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: btn.hovered = true
        onExited:  btn.hovered = false
        onClicked: LauncherService.toggle()
    }
}
