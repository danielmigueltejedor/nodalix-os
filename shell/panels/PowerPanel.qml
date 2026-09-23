import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import "../theme"
import "../services"
import "../widgets/bar"

Item {
    id: root

    // Use a safe minimum for each view. This lets the confirmation become
    // compact while avoiding the one-frame zero-height layout that previously
    // clipped its buttons during the transition.
    implicitWidth:  220
    implicitHeight: (_confirm === ""
        ? Math.max(221, _col.implicitHeight)
        : Math.max(104, _confirmCol.implicitHeight))
        + ThemeManager.spacingLg * 2

    // Pending dangerous action awaiting confirmation ("" | "reboot" | "shutdown")
    property string _confirm: ""

    // Reset confirm state when the power popout closes
    Connections {
        target: PopoutService
        function onCurrentNameChanged() {
            if (PopoutService.currentName !== "power") root._confirm = ""
        }
    }

    // ── Processes ─────────────────────────────────────────────────────────────
    // Lock uses the custom WlSessionLock (LockService), not hyprlock.
    property Process _suspendProc:   Process { command: ["systemctl", "suspend"] }
    property Process _hibernateProc: Process { command: ["systemctl", "hibernate"] }
    // Hyprland 0.56's Lua dispatcher syntax replaces the legacy `exit` name.
    property Process _logoutProc:    Process { command: ["hyprctl", "dispatch", "hl.dsp.exit()"] }
    property Process _rebootProc:    Process { command: ["systemctl", "reboot"] }
    property Process _shutdownProc:  Process { command: ["systemctl", "poweroff"] }

    // ── Action list ─────────────────────────────────────────────────────────--
    ColumnLayout {
        id: _col
        visible: root._confirm === ""
        anchors {
            top: parent.top; left: parent.left; right: parent.right
            margins: ThemeManager.spacingLg
        }
        spacing: 2

        PowerAction { iconState: "lock"; label: I18n.tr("Lock");       onTriggered: LockService.lock() }
        PowerAction { iconState: "suspend"; label: I18n.tr("Suspend");    onTriggered: root._suspendProc.running   = true }
        PowerAction { iconState: "hibernate"; label: I18n.tr("Hibernate");  onTriggered: root._hibernateProc.running = true }

        Rectangle {
            Layout.fillWidth: true; height: 1
            color: ThemeManager.outlineVariant; opacity: 0.35
            Layout.topMargin: 2; Layout.bottomMargin: 2
        }

        PowerAction { iconState: "logout"; label: I18n.tr("Log out");   danger: true; onTriggered: root._confirm = "logout" }
        PowerAction { iconState: "reboot"; label: I18n.tr("Reboot");    danger: true; onTriggered: root._confirm = "reboot" }
        PowerAction { iconState: "shutdown"; label: I18n.tr("Shut down"); danger: true; onTriggered: root._confirm = "shutdown" }
    }

    // ── Confirmation view ─────────────────────────────────────────────────────
    ColumnLayout {
        id: _confirmCol
        visible: root._confirm !== ""
        anchors {
            top: parent.top; left: parent.left; right: parent.right
            margins: ThemeManager.spacingLg
        }
        spacing: 8

        ShellIcon {
            Layout.alignment: Qt.AlignHCenter
            role: "power.action"
            state: root._confirm
            color: ThemeManager.error
            iconSize: 28
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: I18n.tr(root._confirm === "reboot" ? "Reboot now?"
                : (root._confirm === "shutdown" ? "Shut down now?" : "Log out now?"))
            color: ThemeManager.onSurface
            font.family: ThemeManager.fontFor(text)
            font.pixelSize: ThemeManager.fontSizeMd; font.weight: Font.Medium
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4
            spacing: 6
            ConfirmBtn {
                label: I18n.tr("Cancel")
                onClicked: root._confirm = ""
            }
            ConfirmBtn {
                label: I18n.tr("Confirm"); danger: true
                onClicked: {
                    if      (root._confirm === "reboot")   root._rebootProc.running   = true
                    else if (root._confirm === "shutdown") root._shutdownProc.running = true
                    else if (root._confirm === "logout")   root._logoutProc.running   = true
                    root._confirm = ""
                }
            }
        }
    }

    // ── M3 action row ─────────────────────────────────────────────────────────
    component PowerAction: Item {
        id: pa

        property string iconState: ""
        property string label:  ""
        property bool   danger: false

        signal triggered()

        Layout.fillWidth: true
        implicitHeight: 34

        readonly property color _accent: danger ? ThemeManager.error : ThemeManager.primary

        Rectangle {
            anchors.fill: parent; radius: 8
            color: _hov.hovered
                   ? Qt.rgba(pa._accent.r, pa._accent.g, pa._accent.b, 0.1)
                   : Qt.rgba(pa._accent.r, pa._accent.g, pa._accent.b, 0)
        }

        RowLayout {
            anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
            spacing: 10

            ShellIcon {
                role: "power.action"
                state: pa.iconState
                color: _hov.hovered ? pa._accent : ThemeManager.onSurfaceVariant
                iconSize: 16
                Layout.alignment: Qt.AlignVCenter
            }
            Text {
                text:             pa.label
                color:            _hov.hovered ? pa._accent : ThemeManager.onSurfaceVariant
                font.family: ThemeManager.fontFor(text)
                font.pixelSize:   ThemeManager.fontSizeSm
                font.weight:      Font.Medium
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                elide: Text.ElideRight
            }
        }

        HoverHandler { id: _hov; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: pa.triggered() }
    }

    // ── Confirm / cancel button ───────────────────────────────────────────────
    component ConfirmBtn: Rectangle {
        id: cb
        property string label:  ""
        property bool   danger: false
        signal clicked()

        Layout.fillWidth: true
        implicitHeight: 32
        radius: ThemeManager.chipRadius
        readonly property color _accent: danger ? ThemeManager.error : ThemeManager.primary
        color: _cbHov.hovered
            ? Qt.rgba(_accent.r, _accent.g, _accent.b, danger ? 0.9 : 0.18)
            : (danger ? Qt.rgba(_accent.r, _accent.g, _accent.b, 0.75)
                      : ThemeManager.surfaceContainerHigh)

        Text {
            anchors.centerIn: parent
            text: cb.label
            color: cb.danger ? ThemeManager.onError : ThemeManager.onSurface
            font.family: ThemeManager.fontFor(text)
            font.pixelSize: ThemeManager.fontSizeSm; font.weight: Font.Medium
        }
        HoverHandler { id: _cbHov; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: cb.clicked() }
    }
}
