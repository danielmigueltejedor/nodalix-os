import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland
import "../services"
import "../theme"

PanelWindow {
    id: root
    screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name) || Quickshell.screens[0]
    visible: ShortcutGuideService.open
    implicitWidth: 600
    implicitHeight: Math.min(640, content.implicitHeight + 48)
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    component GuideButton: Button {
        implicitHeight: 34
        leftPadding: 12; rightPadding: 12
        background: Rectangle { radius: 9; color: parent.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainer }
        contentItem: Text { text: parent.text; color: ThemeManager.onSurface; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    }
    Rectangle {
        anchors.fill: parent
        radius: 22
        color: ThemeManager.surface
        border.color: ThemeManager.primary
        focus: true
        Keys.onEscapePressed: ShortcutGuideService.open = false
        ColumnLayout {
            id: content
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 24 }
            spacing: 16
            RowLayout {
                Text { text: I18n.tr("Getting around Nodalix"); color: ThemeManager.onSurface; font.pixelSize: 24; Layout.fillWidth: true }
                GuideButton { text: "×"; Accessible.name: I18n.tr("Close"); onClicked: ShortcutGuideService.open = false }
            }
            Text { text: I18n.tr("Super is the Windows or Command key on your keyboard."); color: ThemeManager.onSurfaceVariant; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Repeater {
                model: BindingService.actions.filter(a => BindingService.combo(a.key) !== "")
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    Text { text: modelData.label; color: ThemeManager.onSurface; Layout.fillWidth: true }
                    Text { text: BindingService.combo(modelData.key); color: ThemeManager.primary; font.bold: true }
                }
            }
            Repeater {
                model: [
                    ["Super + ← / →", I18n.tr("Previous / next window or Flow column")],
                    ["Super + Shift + ← / →", I18n.tr("Move window left / right")],
                    ["Super + 1…9", I18n.tr("Switch workspace")],
                    [I18n.tr("Super + mouse drag"), I18n.tr("Move a window")]
                ]
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    Text { text: modelData[1]; color: ThemeManager.onSurface; Layout.fillWidth: true }
                    Text { text: modelData[0]; color: ThemeManager.primary; font.bold: true }
                }
            }
            Text { text: I18n.tr("In Flow, windows extend horizontally. Use the arrows in the dock or Super + Left / Right."); color: ThemeManager.onSurfaceVariant; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            GuideButton { text: I18n.tr("Customize shortcuts"); onClicked: { ShortcutGuideService.open = false; SettingsUi.category = "keybindings"; SettingsUi.show() } }
        }
    }
}
