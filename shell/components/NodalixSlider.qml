import QtQuick
import "../theme"

FocusScope {
    id: root

    property real value: 0.0
    property real step: 0.05
    property string accessibleName: ""
    property color accentColor: ThemeManager.primary
    property color trackColor: ThemeManager.outlineVariant

    signal moved(real value)
    signal committed()

    readonly property real fraction:
        Math.max(0.0, Math.min(1.0, root.value))

    readonly property int trackHeight: 20

    implicitWidth: 160
    implicitHeight: 36

    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName

    function setFromFraction(fraction) {
        root.moved(Math.max(0.0, Math.min(1.0, fraction)))
    }

    function nudge(direction) {
        const amount = Math.max(0.001, root.step)
        root.setFromFraction(root.fraction + direction * amount)
    }

    Keys.onPressed: event => {
        if (!root.enabled)
            return

        if (event.key === Qt.Key_Left || event.key === Qt.Key_Down) {
            root.nudge(-1)
            root.committed()
            event.accepted = true
        } else if (event.key === Qt.Key_Right || event.key === Qt.Key_Up) {
            root.nudge(1)
            root.committed()
            event.accepted = true
        } else if (event.key === Qt.Key_Home) {
            root.setFromFraction(0)
            root.committed()
            event.accepted = true
        } else if (event.key === Qt.Key_End) {
            root.setFromFraction(1)
            root.committed()
            event.accepted = true
        }
    }

    Rectangle {
        id: track

        anchors {
            left: parent.left
            right: parent.right
            verticalCenter: parent.verticalCenter
        }

        height: root.trackHeight
        radius: height / 2
        clip: true
        antialiasing: true

        color: root.trackColor

        border.width: root.activeFocus ? 2 : (_mouse.containsMouse ? 1 : 0)
        border.color: root.activeFocus
            ? root.accentColor
            : Qt.rgba(
                ThemeManager.onSurface.r,
                ThemeManager.onSurface.g,
                ThemeManager.onSurface.b,
                0.18
            )

        Behavior on border.width {
            NumberAnimation { duration: 100 }
        }

        Rectangle {
            anchors {
                left: parent.left
                top: parent.top
                bottom: parent.bottom
            }

            width: parent.width * root.fraction
            radius: parent.radius
            antialiasing: true
            color: root.accentColor

            Behavior on width {
                NumberAnimation {
                    duration: 70
                    easing.type: Easing.OutCubic
                }
            }
        }
    }

    MouseArea {
        id: _mouse

        anchors.fill: parent
        hoverEnabled: true
        preventStealing: true
        cursorShape: root.enabled
            ? Qt.PointingHandCursor
            : Qt.ArrowCursor

        enabled: root.enabled

        function apply(x) {
            root.forceActiveFocus()
            root.setFromFraction(x / Math.max(1, width))
        }

        onPressed: event => apply(event.x)
        onReleased: root.committed()

        onPositionChanged: event => {
            if (pressed)
                apply(event.x)
        }
    }
}
