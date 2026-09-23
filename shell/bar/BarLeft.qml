import QtQuick
import QtQuick.Layouts
import "../theme"
import "../services"
import "../widgets/bar"

RowLayout {
    id: root

    required property var barScreen

    spacing: ThemeManager.spacing

    readonly property bool _showLauncher: SettingsService.get("bar.widgets.launcher", true)
    readonly property bool _showTitle:    SettingsService.get("bar.widgets.windowTitle", true)

    LauncherButton { visible: root._showLauncher }

    WindowTitle { visible: root._showTitle }
}
