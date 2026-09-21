import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import "./services"
import "./theme"
import "./panels"

PanelWindow {
    id: root
    required property var modelData
    screen: modelData
    visible: WindowLayoutService.mode === "desktop" || WindowLayoutService.mode === "scrolling"
    anchors.bottom: true
    margins.bottom: 12
    readonly property bool expanded: LauncherService.open && LauncherService.screenName === modelData.name
    readonly property real dockWidth: Math.min(modelData.width - 40, Math.max(380, tasks.implicitWidth + (WindowLayoutService.mode === "scrolling" ? 360 : 260)))
    readonly property real drawerHeight: Math.min(640, modelData.height - 150)
    implicitWidth: expanded ? Math.max(dockWidth, Math.min(660, modelData.width - 48)) : dockWidth
    implicitHeight: expanded ? drawerHeight + 56 : 64
    exclusiveZone: 80
    color: "transparent"
    WlrLayershell.layer: expanded ? WlrLayer.Overlay : WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    HyprlandFocusGrab { windows: [root]; active: root.expanded; onCleared: LauncherService.hide() }
    Rectangle {
        id: drawer
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; bottomMargin: 52 }
        height: root.expanded ? root.drawerHeight : 0
        visible: root.expanded
        radius: 26
        color: ThemeManager.surface
        border.color: ThemeManager.outlineVariant
        clip: true
        Loader { anchors.fill: parent; active: root.expanded; sourceComponent: Launcher { active: root.expanded } }
    }
    Rectangle {
        anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter }
        width: root.dockWidth; height: 64
        radius: 21
        color: ThemeManager.surface
        border.color: ThemeManager.outlineVariant
        RowLayout {
            anchors { fill: parent; margins: 8 }
            spacing: 6
            DockButton { text: "☰"; Accessible.name: I18n.tr("App launcher"); highlighted: root.expanded; onClicked: LauncherService.toggle(root.modelData.name) }
            DockButton { text: "▦"; Accessible.name: I18n.tr("Window overview"); onClicked: Quickshell.execDetached(["python3", Paths.configDir + "/scripts/nodalix-overview.py"]) }
            DockButton { visible: WindowLayoutService.mode === "scrolling"; text: "←"; Accessible.name: I18n.tr("Previous column"); onClicked: Hyprland.dispatch('hl.dsp.focus({direction="l"})') }
            DockButton { visible: WindowLayoutService.mode === "scrolling"; text: "→"; Accessible.name: I18n.tr("Next column"); onClicked: Hyprland.dispatch('hl.dsp.focus({direction="r"})') }
            ScrollView {
                Layout.fillWidth: true; Layout.fillHeight: true
                clip: true
                contentWidth: tasks.implicitWidth
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff
                Row {
                    id: tasks
                    spacing: 5
                    Repeater {
                        model: DockService.groups
                        delegate: DockButton {
                            required property var modelData
                            width: 46; height: 46
                            fallbackText: modelData.name.slice(0, 1).toUpperCase()
                            icon.source: modelData.app ? AppService.iconFor(modelData.app) : ""
                            highlighted: modelData.windows.indexOf(Hyprland.activeToplevel) >= 0
                            Accessible.name: modelData.name
                            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; visible: parent.modelData.windows.length > 0; width: parent.highlighted ? 12 : 4; height: 3; radius: 2; color: ThemeManager.primary }
                            onClicked: { LauncherService.hide(); DockService.activate(modelData, root.modelData.name) }
                        }
                    }
                }
            }
            DockButton { text: "−"; enabled: !!Hyprland.activeToplevel; Accessible.name: I18n.tr("Minimize active window"); onClicked: DockService.run("minimize", Hyprland.activeToplevel, root.modelData.name) }
            DockButton { text: "□"; enabled: !!Hyprland.activeToplevel; Accessible.name: I18n.tr("Maximize active window"); onClicked: DockService.run("maximize", Hyprland.activeToplevel, root.modelData.name) }
            DockButton { text: "?"; Accessible.name: I18n.tr("Shortcut guide"); onClicked: { LauncherService.hide(); ShortcutGuideService.open = true } }
        }
    }
    component DockButton: Button {
        id: button
        property string fallbackText: "◇"
        implicitWidth: 42; implicitHeight: 46
        background: Rectangle { radius: 13; color: button.down ? ThemeManager.primary : (button.hovered || button.highlighted ? ThemeManager.surfaceContainerHigh : "transparent"); Behavior on color { ColorAnimation { duration: 120 } } }
        contentItem: Item {
            Image { id: appIcon; anchors.centerIn: parent; width: 30; height: 30; source: button.icon.source; visible: status === Image.Ready; fillMode: Image.PreserveAspectFit }
            Text { anchors.centerIn: parent; text: button.text || button.fallbackText; visible: button.text !== "" || appIcon.status !== Image.Ready; color: ThemeManager.onSurface; font.family: ThemeManager.fontFor(text); font.pixelSize: 21 }
        }
    }
}
