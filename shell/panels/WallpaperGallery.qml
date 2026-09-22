import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Widgets
import "../services"
import "../theme"
import "../components"

ColumnLayout {
    id: root
    spacing: 14
    focus: ToolsService.wpOpen
    Keys.onPressed: function(event) {
        switch (event.key) {
            case Qt.Key_Left: ToolsService.moveSelection(-1); break
            case Qt.Key_Right: ToolsService.moveSelection(1); break
            case Qt.Key_Up: ToolsService.up(); break
            case Qt.Key_Down: ToolsService.down(); break
            case Qt.Key_Return:
            case Qt.Key_Enter: ToolsService.activate(); break
            case Qt.Key_Escape: ToolsService.close(); break
            default: return
        }
        event.accepted = true
    }
    RowLayout {
        Layout.fillWidth: true
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3
            Text { text: I18n.tr("Wallpaper collection"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFor(text); font.pixelSize: 22; font.weight: Font.DemiBold }
            Text { text: I18n.tr("Choose a background for your desktop"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text); font.pixelSize: 12 }
        }
        Item { Layout.fillWidth: true }
        ActionButton { text: "×"; Accessible.name: I18n.tr("Close"); onClicked: ToolsService.close() }
    }
    TextField {
        Layout.fillWidth: true
        implicitHeight: 42
        placeholderText: I18n.tr("Search wallpapers")
        color: ThemeManager.onSurface
        placeholderTextColor: ThemeManager.onSurfaceVariant
        font.family: ThemeManager.fontFor(text)
        text: ToolsService.wpQuery
        onTextEdited: ToolsService.wpQuery = text
        leftPadding: 14
        background: Rectangle { radius: 13; color: ThemeManager.surfaceContainer }
        Keys.onEscapePressed: ToolsService.close()
    }
    RowLayout {
        spacing: 6
        Repeater {
            model: [{key:"all",label:"All"},{key:"static",label:"Static"},{key:"animated",label:"Animated"},{key:"favorites",label:"Favorites"}]
            ActionButton {
                required property var modelData
                text: I18n.tr(modelData.label)
                emphasized: ToolsService.wpFilter === modelData.key
                onClicked: { ToolsService.wpFilter = modelData.key; ToolsService.wpSelected = 0 }
            }
        }
    }
    GridView {
        id: grid
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: ToolsService.wpEntries
        cellWidth: width / Math.max(1, Math.floor(width / 165))
        cellHeight: cellWidth * 0.65 + 30
        currentIndex: ToolsService.wpSelected
        onCurrentIndexChanged: positionViewAtIndex(currentIndex, GridView.Contain)
        onWidthChanged: ToolsService.wpColumns = Math.max(1, Math.floor(width / 165))
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        delegate: Item {
            id: tile
            required property var modelData
            required property int index
            width: grid.cellWidth
            height: grid.cellHeight
            readonly property bool selected: ToolsService.wpSelected === index
            ClippingRectangle {
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 5 }
                height: grid.cellWidth * 0.65
                radius: 14
                color: ThemeManager.surfaceContainerHigh
                Image {
                    anchors.fill: parent
                    source: "file://" + (tile.modelData.animated ? WallpaperService.thumbnailFor(tile.modelData.path) : tile.modelData.path)
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    sourceSize.width: 420
                }
                Rectangle { anchors.fill: parent; color: Qt.rgba(0,0,0,0.12); visible: tile.selected }
                TapHandler { onTapped: ToolsService.wpSelected = tile.index; onDoubleTapped: ToolsService.commitWallpaper(tile.modelData) }
                Rectangle {
                    anchors { left: parent.left; bottom: parent.bottom; margins: 8 }
                    width: badge.implicitWidth + 14; height: 24; radius: 8
                    color: Qt.rgba(0,0,0,0.6)
                    visible: tile.modelData.animated || tile.selected
                    Text { id: badge; anchors.centerIn: parent; text: tile.selected ? "✓" : "▶"; color: "white"; font.pixelSize: 13 }
                }
                Button {
                    anchors { right: parent.right; top: parent.top; margins: 7 }
                    width: 30; height: 30
                    Accessible.name: I18n.tr("Favorites")
                    background: Rectangle { radius: 15; color: Qt.rgba(0,0,0,0.55) }
                    contentItem: Text { text: WallpaperService.isFavorite(tile.modelData.path) ? "♥" : "♡"; color: "white"; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter; font.pixelSize: 18 }
                    onClicked: WallpaperService.toggleFavorite(tile.modelData.path)
                }
            }
            Text {
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 7 }
                text: WallpaperService.displayName(tile.modelData.path)
                elide: Text.ElideRight
                color: tile.selected ? ThemeManager.onSurface : ThemeManager.onSurfaceVariant
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: 12
            }
        }
        Text { anchors.centerIn: parent; visible: grid.count === 0; text: I18n.tr("No matching wallpapers"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text) }
    }
    RowLayout {
        Layout.fillWidth: true
        Text { Layout.fillWidth: true; text: I18n.tr("Select to preview · Enter to apply"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text); font.pixelSize: 12; wrapMode: Text.WordWrap }
        ActionButton { text: I18n.tr("Apply"); emphasized: true; enabled: grid.count > 0; onClicked: ToolsService.commitWallpaper(ToolsService.wpEntries[ToolsService.wpSelected]) }
    }
}
