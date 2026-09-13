import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import "../theme"
import "../services"
import "../widgets/bar"

Item {
    id: root

    readonly property int _pad: ThemeManager.spacing

    // Width = widest item (from TextMetrics via Layout.minimumWidth), capped 140–400
    implicitWidth:  Math.max(140, Math.min(_page.implicitWidth + _pad * 2, 400))
    implicitHeight: _page.implicitHeight + _pad * 2

    Flickable {
        id: _flick
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: _page.implicitHeight + root._pad * 2
        boundsBehavior: Flickable.StopAtBounds
        interactive: contentHeight > height

        ScrollBar.vertical: ScrollBar {
            policy: _flick.contentHeight > _flick.height
                ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            width: 4
        }

        TrayMenuPage {
            id: _page
            width: _flick.width - root._pad * 2
            x: root._pad
            y: root._pad
        }
    }
}
