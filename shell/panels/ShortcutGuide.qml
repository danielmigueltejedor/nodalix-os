import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland
import "../components"
import "../services"
import "../theme"

PanelWindow {
    id: root
    screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name) || Quickshell.screens[0]
    visible: ShortcutGuideService.open
    implicitWidth: Math.min(580, (screen?.width || 640) - 48)
    implicitHeight: Math.min(760, (screen?.height || 800) - 80)
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    Rectangle {
        anchors.fill: parent
        radius: 28
        color: ThemeManager.surface
        border.color: Qt.rgba(ThemeManager.outline.r, ThemeManager.outline.g, ThemeManager.outline.b, 0.2)
        focus: true
        Keys.onEscapePressed: ShortcutGuideService.open = false
        ColumnLayout {
            anchors { fill: parent; margins: 28 }
            spacing: 20
            RowLayout {
                Layout.fillWidth: true
                ColumnLayout {
                    spacing: 7
                    Layout.fillWidth: true
                    Text { text: "N O D A L I X"; color: ThemeManager.primary; font.family: ThemeManager.fontFor(text); font.pixelSize: 11; font.weight: Font.DemiBold }
                    Text { text: I18n.tr("Make yourself at home"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFor(text); font.pixelSize: 27; font.weight: Font.Medium }
                }
                ActionButton { text: "×"; Accessible.name: I18n.tr("Close"); onClicked: ShortcutGuideService.open = false }
            }
            Text {
                Layout.fillWidth: true
                text: I18n.tr("A few shortcuts. Everything else is within reach of your dock.")
                color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text); font.pixelSize: 13
                wrapMode: Text.WordWrap
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: availableWidth
                ColumnLayout {
                    width: parent.width
                    spacing: 8
                    Repeater {
                        model: BindingService.actions.filter(a => BindingService.combo(a.key) !== "" && a.key !== "shortcuts")
                        delegate: ShortcutRow { required property var modelData; title: modelData.label; keys: BindingService.combo(modelData.key) }
                    }
                    Text { Layout.topMargin: 14; Layout.bottomMargin: 4; text: I18n.tr("Move with ease"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text); font.pixelSize: 12; font.weight: Font.Medium }
                    ShortcutRow { title: I18n.tr("Previous / next window or Flow column"); keys: "Super + ← / →" }
                    ShortcutRow { title: I18n.tr("Switch workspace"); keys: "Super + 1…9" }
                    ShortcutRow { title: I18n.tr("Move a window"); keys: I18n.tr("Super + drag") }
                    ShortcutRow { title: I18n.tr("Resize a window"); keys: I18n.tr("Super + right drag") }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: ThemeManager.outlineVariant; opacity: 0.35 }
            Text { Layout.fillWidth: true; text: I18n.tr("Super is the Windows or Command key on your keyboard."); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFor(text); font.pixelSize: 12; wrapMode: Text.WordWrap }
            RowLayout {
                Layout.fillWidth: true
                ActionButton { text: I18n.tr("Customize shortcuts"); onClicked: { ShortcutGuideService.open = false; SettingsUi.category = "keybindings"; SettingsUi.show() } }
                Item { Layout.fillWidth: true }
                ActionButton { text: I18n.tr("Got it"); emphasized: true; onClicked: ShortcutGuideService.open = false }
            }
        }
    }
    component ShortcutRow: Rectangle {
        id: row
        required property string title
        required property string keys
        Layout.fillWidth: true
        implicitHeight: 58
        radius: 15
        color: ThemeManager.surfaceContainer
        RowLayout {
            anchors { fill: parent; margins: 14 }
            spacing: 12
            Text { Layout.fillWidth: true; text: row.title; color: ThemeManager.onSurface; font.family: ThemeManager.fontFor(text); font.pixelSize: 13; wrapMode: Text.WordWrap }
            Rectangle {
                implicitWidth: keyLabel.implicitWidth + 18
                implicitHeight: 29
                radius: 7
                color: ThemeManager.surfaceContainerHigh
                border.color: Qt.rgba(ThemeManager.outline.r, ThemeManager.outline.g, ThemeManager.outline.b, 0.15)
                Text { id: keyLabel; anchors.centerIn: parent; text: row.keys.replace(/SUPER/g, "Super").replace(/SPACE/g, I18n.language === "es" ? "Espacio" : "Space").replace(/TAB/g, "Tab"); color: ThemeManager.primary; font.family: ThemeManager.fontFor(text); font.pixelSize: 11; font.weight: Font.Medium }
            }
        }
    }
}
