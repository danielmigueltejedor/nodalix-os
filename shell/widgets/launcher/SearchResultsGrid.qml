import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell.Widgets
import "../../theme"
import "../../services"
import "../bar" as Bar

// Virtualized launcher search grid.
// Unlike AppGrid's Repeater, GridView only instantiates the visible result
// delegates (+ one small cache row), avoiding a first-search burst of up to
// forty complete app tiles and icon items.
ColumnLayout {
    id: root

    property string title: ""
    property var apps: []
    property int columns: 6
    property int selectedIndex: -1
    property real tileH: 92
    property real maxViewportHeight: 390

    signal activated(var app)
    signal contextRequested(var app, real mx, real my)

    spacing: 8
    implicitHeight: (_title.visible ? _title.implicitHeight + spacing : 0) + _grid.implicitHeight

    Text {
        id: _title
        visible: root.title.length > 0
        text: root.title
        color: ThemeManager.onSurfaceVariant
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeSm
        font.bold: true
    }

    GridView {
        id: _grid

        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(
            root.maxViewportHeight,
            Math.max(root.tileH, contentHeight)
        )
        implicitHeight: Layout.preferredHeight

        clip: true
        model: root.apps
        cellWidth: width / Math.max(1, root.columns)
        cellHeight: root.tileH + 6
        boundsBehavior: Flickable.StopAtBounds
        interactive: contentHeight > height
        reuseItems: true
        cacheBuffer: cellHeight
        keyNavigationEnabled: false
        currentIndex: root.selectedIndex

        ScrollBar.vertical: ScrollBar {
            policy: _grid.contentHeight > _grid.height
                ? ScrollBar.AsNeeded
                : ScrollBar.AlwaysOff
            width: 4
        }

        delegate: Item {
            id: cell
            required property var modelData
            required property int index

            width: _grid.cellWidth
            height: _grid.cellHeight

            Component {
                id: _appIconComponent
                IconImage {
                    implicitSize: 40
                    source: AppService.iconFor(cell.modelData)
                }
            }

            Component {
                id: _systemIconComponent
                Bar.ShellIcon {
                    iconName: cell.modelData?._nodalixIconName ?? ""
                    color: ThemeManager.primary
                    iconSize: 30
                }
            }

            Rectangle {
                id: tile
                width: Math.max(0, parent.width - 6)
                height: root.tileH
                radius: ThemeManager.chipRadius

                readonly property bool _selected: root.selectedIndex === cell.index
                color: _selected
                    ? ThemeManager.secondaryContainer
                    : (_mouse.containsMouse ? ThemeManager.surfaceContainerHigh : "transparent")
                border.width: _selected ? 2 : 0
                border.color: ThemeManager.primary

                MouseArea {
                    id: _mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: Qt.PointingHandCursor

                    onClicked: (mouse) => {
                        if (mouse.button === Qt.RightButton) {
                            const p = mapToItem(null, mouse.x, mouse.y)
                            root.contextRequested(cell.modelData, p.x, p.y)
                        } else if (mouse.button === Qt.LeftButton) {
                            root.activated(cell.modelData)
                        }
                    }
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 6

                    Loader {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.preferredWidth: 40
                        Layout.preferredHeight: 40
                        sourceComponent: (cell.modelData?._nodalixIconName ?? "") !== ""
                            ? _systemIconComponent
                            : _appIconComponent
                    }

                    Text {
                        Layout.fillWidth: true
                        text: cell.modelData?.name ?? ""
                        color: ThemeManager.onSurface
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        maximumLineCount: 2
                        wrapMode: Text.Wrap
                    }
                }
            }
        }

        Connections {
            target: root
            function onSelectedIndexChanged() {
                if (root.selectedIndex >= 0 && root.selectedIndex < root.apps.length)
                    _grid.positionViewAtIndex(root.selectedIndex, GridView.Contain)
            }
        }
    }
}
