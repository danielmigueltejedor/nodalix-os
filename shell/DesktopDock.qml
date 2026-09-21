import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import "./services"
import "./theme"

PanelWindow {
    id: root
    required property var modelData
    screen: modelData
    visible: WindowLayoutService.mode === "desktop" || WindowLayoutService.mode === "scrolling"
    anchors.bottom: true
    margins.bottom: 12
    implicitWidth: Math.min(modelData.width - 32, Math.max(430, tasks.implicitWidth + (WindowLayoutService.mode === "scrolling" ? 360 : 260)))
    implicitHeight: 64
    exclusiveZone: 80
    color: "transparent"
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    property var windows: Hyprland.toplevels.values.filter(w => w.monitor?.name === modelData.name || w.workspace?.name === "special:nodalix-minimized")
    readonly property var runningKeys: windows.map(w => AppService.keyOf(AppService.byClass(w.lastIpcObject?.class || w.appId || "")))
    function focusWindow(win) {
        const address = win.lastIpcObject?.address || win.address
        if (win.workspace?.name === "special:nodalix-minimized") {
            const ws = Hyprland.monitorFor(root.modelData)?.activeWorkspace?.id || 1
            Hyprland.dispatch('hl.dsp.window.move({workspace=' + ws + ',silent=true,window="address:' + address + '"})')
        }
        Hyprland.dispatch('hl.dsp.focus({window="address:' + address + '"})')
    }
    component DockButton: Button {
        id: dockButton
        property string fallbackText: "◇"
        implicitWidth: 44
        implicitHeight: 46
        background: Rectangle {
            radius: 12
            color: parent.down ? ThemeManager.primary : (parent.hovered || parent.highlighted ? ThemeManager.surfaceContainerHigh : "transparent")
        }
        contentItem: Item {
            Image { id: appIcon; anchors.centerIn: parent; width: 30; height: 30; source: dockButton.icon.source; visible: status === Image.Ready; fillMode: Image.PreserveAspectFit }
            Text { anchors.centerIn: parent; visible: dockButton.text === "" && appIcon.status !== Image.Ready; text: dockButton.fallbackText; color: ThemeManager.onSurface; font.family: ThemeManager.fontFor(text); font.pixelSize: 21 }
            Text { anchors.centerIn: parent; text: parent.parent.text; color: ThemeManager.onSurface; font.pixelSize: 22; visible: text !== "" }
        }
    }
    Rectangle {
        anchors.fill: parent
        radius: 20
        color: ThemeManager.surface
        border.color: ThemeManager.outlineVariant
        RowLayout {
            anchors { fill: parent; margins: 8 }
            spacing: 6
            DockButton { text: "☰"; Accessible.name: I18n.tr("App launcher"); ToolTip.visible: hovered; ToolTip.text: I18n.tr("App launcher") + " · " + BindingService.combo("launcher"); onClicked: LauncherService.toggle() }
            DockButton { text: "▦"; Accessible.name: I18n.tr("Window overview"); ToolTip.visible: hovered; ToolTip.text: I18n.tr("Window overview") + " · " + BindingService.combo("overview"); onClicked: Quickshell.execDetached(["nodalix-overview"]) }
            DockButton { visible: WindowLayoutService.mode === "scrolling"; text: "←"; Accessible.name: I18n.tr("Previous column"); onClicked: Hyprland.dispatch('hl.dsp.focus({direction="l"})') }
            DockButton { visible: WindowLayoutService.mode === "scrolling"; text: "→"; Accessible.name: I18n.tr("Next column"); onClicked: Hyprland.dispatch('hl.dsp.focus({direction="r"})') }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: tasks.implicitWidth
                Row {
                    id: tasks
                    spacing: 5
                    Repeater {
                        model: root.windows
                        delegate: DockButton {
                            required property var modelData
                            width: 46; height: 46
                            readonly property var app: AppService.byClass(modelData.lastIpcObject?.class || modelData.appId || "")
                            fallbackText: (app?.name || modelData.title || "◇").slice(0, 1).toUpperCase()
                            icon.source: app ? AppService.iconFor(app) : ""
                            icon.width: 30; icon.height: 30
                            icon.color: "transparent"
                            highlighted: modelData === Hyprland.activeToplevel
                            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; width: parent.highlighted ? 12 : 4; height: 3; radius: 2; color: ThemeManager.primary }
                            Accessible.name: modelData.title || app?.name || I18n.tr("Window")
                            ToolTip.visible: hovered
                            ToolTip.text: Accessible.name
                            onClicked: root.focusWindow(modelData)
                        }
                    }
                    Repeater {
                        model: PinnedService.pinned.slice(0, 5).map(key => AppService.byKey(key)).filter(app => app !== null && root.runningKeys.indexOf(AppService.keyOf(app)) < 0)
                        delegate: DockButton {
                            required property var modelData
                            width: 46; height: 46
                            icon.source: AppService.iconFor(modelData)
                            icon.color: "transparent"
                            icon.width: 30; icon.height: 30
                            Accessible.name: modelData.name
                            ToolTip.visible: hovered; ToolTip.text: modelData.name
                            onClicked: AppService.launch(modelData)
                        }
                    }
                }
            }
            DockButton { text: "−"; enabled: !!Hyprland.activeToplevel; Accessible.name: I18n.tr("Minimize active window"); ToolTip.visible: hovered; ToolTip.text: Accessible.name; onClicked: Hyprland.dispatch('hl.dsp.window.move({workspace="special:nodalix-minimized",silent=true})') }
            DockButton { text: "□"; enabled: !!Hyprland.activeToplevel; Accessible.name: I18n.tr("Maximize active window"); ToolTip.visible: hovered; ToolTip.text: Accessible.name; onClicked: Hyprland.dispatch('hl.dsp.window.fullscreen({mode="maximized",action="toggle"})') }
            DockButton { text: "?"; Accessible.name: I18n.tr("Shortcut guide"); ToolTip.visible: hovered; ToolTip.text: I18n.tr("Shortcut guide") + " · " + BindingService.combo("shortcuts"); onClicked: ShortcutGuideService.open = true }
        }
    }
}
