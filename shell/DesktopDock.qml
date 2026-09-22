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
    margins.bottom: 0
    readonly property bool expanded: LauncherService.open && LauncherService.screenName === modelData.name
    readonly property real dockWidth: Math.min(modelData.width - 40, Math.max(380, tasks.implicitWidth + (WindowLayoutService.mode === "scrolling" ? 360 : 260)))
    readonly property real drawerHeight: Math.min(640, modelData.height - 150)
    property real reveal: expanded ? 1 : 0
    Behavior on reveal { NumberAnimation { duration: 300; easing.type: Easing.Bezier; easing.bezierCurve: root.expanded ? [0.05,0.7,0.1,1,1,1] : [0.3,0,0.8,0.15,1,1] } }
    readonly property bool drawerVisible: expanded || reveal > 0.001
    implicitWidth: drawerVisible ? Math.max(dockWidth, Math.min(660, modelData.width - 48)) : dockWidth
    implicitHeight: 64 + drawerHeight * reveal
    exclusiveZone: 72
    color: "transparent"
    WlrLayershell.layer: drawerVisible ? WlrLayer.Overlay : WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    HyprlandFocusGrab { windows: [root]; active: root.expanded; onCleared: LauncherService.hide() }
    Rectangle {
        id: drawer
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; bottomMargin: 48 }
        height: root.drawerHeight * root.reveal + 16
        visible: root.drawerVisible
        radius: 26
        color: ThemeManager.surface
        clip: true
        Loader { anchors { left: parent.left; right: parent.right; top: parent.top } height: root.drawerHeight; active: root.drawerVisible; opacity: root.reveal; sourceComponent: Launcher { active: root.expanded } }
    }
    Rectangle {
        anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter }
        width: root.dockWidth; height: 64
        radius: 21
        color: ThemeManager.surface
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 24; color: ThemeManager.surface }
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
