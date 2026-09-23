import QtQuick
import QtQuick.Controls
import "../theme"

Button {
    id: root
    property bool emphasized: false
    property bool destructive: false
    property bool glass: false
    implicitHeight: 44
    implicitWidth: Math.max(44, label.implicitWidth + 32)
    leftPadding: 16
    rightPadding: 16
    Accessible.name: text
    background: Rectangle {
        radius: 14
        color: root.emphasized ? (root.destructive ? ThemeManager.error : ThemeManager.primary)
            : (root.glass ? Qt.rgba(1, 1, 1, root.hovered ? 0.18 : 0.09)
            : (root.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainer))
        border.width: root.activeFocus || root.glass ? 1 : 0
        border.color: root.activeFocus ? ThemeManager.primary : Qt.rgba(1, 1, 1, 0.16)
        opacity: root.enabled ? (root.down ? 0.7 : 1) : 0.4
        Behavior on color { ColorAnimation { duration: 140 } }
    }
    contentItem: Text {
        id: label
        text: root.text
        color: root.emphasized ? (root.destructive ? ThemeManager.onError : ThemeManager.onPrimary)
            : (root.glass ? "white" : ThemeManager.onSurface)
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: 13
        font.weight: Font.Medium
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
