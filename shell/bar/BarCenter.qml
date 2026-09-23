import QtQuick
import QtQuick.Layouts
import "../theme"
import "../widgets/bar"

RowLayout {
    id: root

    required property var barScreen

    spacing: ThemeManager.spacingLg

    Workspaces {
        barScreen: root.barScreen
        Layout.alignment: Qt.AlignVCenter
    }

    Clock {
        barScreen: root.barScreen
        Layout.alignment: Qt.AlignVCenter
    }
}
