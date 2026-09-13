import QtQuick
import QtQuick.Layouts
import "../../theme"
import "../../services"

Rectangle {
    id: root

    property var now: new Date()
    property var barScreen: null
    readonly property color _privacyColor: PrivacyService.cameraActive ? "#34c759"
        : (PrivacyService.microphoneActive ? "#ff9f0a" : "#4da3ff")

    implicitWidth: _content.implicitWidth + 16
    implicitHeight: Math.max(28, _content.implicitHeight + 8)
    radius: implicitHeight / 2
    color: PrivacyService.active
        ? Qt.rgba(_privacyColor.r, _privacyColor.g, _privacyColor.b, 0.14)
        : "transparent"
    border.width: PrivacyService.active ? 1 : 0
    border.color: _privacyColor
    Behavior on color { ColorAnimation { duration: 180 } }
    Behavior on border.color { ColorAnimation { duration: 180 } }

    Timer {
        interval: 1000
        running:  true
        repeat:   true
        onTriggered: root.now = new Date()
    }

    HoverHandler {
        onHoveredChanged: {
            const pos = root.mapToItem(null, root.width / 2, 0)
            if (hovered) {
                PopoutService.open("dashboard", pos.x, root.barScreen)
                PopoutService.widgetHovered = true
            } else {
                PopoutService.widgetHovered = false
            }
        }
    }

    readonly property string _timeFmt: {
        const h24  = SettingsService.get("bar.clock.use24h", true)
        const secs = SettingsService.get("bar.clock.seconds", false)
        return (h24 ? "hh:mm" : "h:mm") + (secs ? ":ss" : "") + (h24 ? "" : " AP")
    }

    RowLayout {
        id: _content
        anchors.centerIn: parent
        spacing: 6

        Text {
            id: timeText
            text: Qt.formatTime(root.now, root._timeFmt)
            color: ThemeManager.onSurface
            font.family: ThemeManager.fontFamily
            font.pixelSize: ThemeManager.fontSizeMd
            font.weight: Font.Medium
        }

        Rectangle {
            width: 1; height: 12
            color: ThemeManager.outlineVariant
            opacity: 0.6
            Layout.alignment: Qt.AlignVCenter
        }

        Text {
            id: dateText
            text: root.now.toLocaleDateString(Qt.locale(I18n.localeName), "ddd d MMM")
            color: ThemeManager.onSurface
            font.family: ThemeManager.fontFamily
            font.pixelSize: ThemeManager.fontSizeSm
            font.weight: Font.Medium
        }

        Row {
            visible: PrivacyService.active
            spacing: 3
            Layout.alignment: Qt.AlignVCenter
            Rectangle {
                visible: PrivacyService.cameraActive
                width: 6; height: 6; radius: 3
                color: "#34c759"
            }
            Rectangle {
                visible: PrivacyService.microphoneActive
                width: 6; height: 6; radius: 3
                color: "#ff9f0a"
            }
            Rectangle {
                visible: PrivacyService.locationActive
                width: 6; height: 6; radius: 3
                color: "#4da3ff"
            }
        }
    }
}
