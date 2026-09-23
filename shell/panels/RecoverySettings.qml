import QtQuick
import QtQuick.Layouts
import "../theme"
import "../services"

ColumnLayout {
    id: root
    spacing: 14
    property string pendingAction: ""
    property string pendingName: ""
    onVisibleChanged: { if (visible) RecoveryService.load() }

    Text {
        text: I18n.tr("Backups and recovery")
        color: ThemeManager.onSurface
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeLg
    }
    Text {
        Layout.fillWidth: true
        text: I18n.tr("Return your system to a working state. Btrfs snapshots share unchanged data and preserve your personal files.")
        wrapMode: Text.WordWrap
        color: ThemeManager.onSurfaceVariant
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeSm
    }
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: summary.implicitHeight + 32
        radius: 18
        color: ThemeManager.surfaceContainerLow
        ColumnLayout {
            id: summary
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
            spacing: 8
            Text {
                text: RecoveryService.data.supported ? I18n.tr("Efficient system snapshots") : I18n.tr("Check your recovery setup")
                color: ThemeManager.onSurface
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeMd
            }
            Text {
                Layout.fillWidth: true
                text: I18n.tr("Includes the system and its boot files. Documents, downloads and personal files need a separate backup. A snapshot on this disk cannot protect against disk failure.")
                wrapMode: Text.WordWrap
                color: ThemeManager.onSurfaceVariant
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeSm
            }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                RecoveryButton { label: I18n.tr("Check setup"); onClicked: RecoveryService.perform("refresh", "", false) }
                RecoveryButton { label: I18n.tr("Create recovery point"); enabled: RecoveryService.data.supported === true && !RecoveryService.data.rebootRequired; onClicked: RecoveryService.perform("create", "", false) }
                RecoveryButton { label: I18n.tr("Advanced recovery"); onClicked: RecoveryService.advanced() }
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        visible: RecoveryService.data.supported === true
        ColumnLayout {
            Layout.fillWidth: true
            Text {
                text: I18n.tr("Daily protection")
                color: ThemeManager.onSurface
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeMd
            }
            Text {
                Layout.fillWidth: true
                text: I18n.tr("Keep the latest three automatic points. Manual points stay until you delete them.")
                wrapMode: Text.WordWrap
                color: ThemeManager.onSurfaceVariant
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeSm
            }
        }
        RecoveryButton {
            label: RecoveryService.data.automatic ? I18n.tr("Disable") : I18n.tr("Enable")
            onClicked: RecoveryService.perform("schedule", RecoveryService.data.automatic ? "off" : "on", false)
        }
    }
    Text {
        visible: RecoveryService.message !== ""
        Layout.fillWidth: true
        text: RecoveryService.message
        wrapMode: Text.WordWrap
        color: ThemeManager.onSurfaceVariant
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeSm
    }
    Text {
        visible: RecoveryService.data.rebootRequired === true
        Layout.fillWidth: true
        text: I18n.tr("Recovery is ready. Save your work and restart to use the restored system.")
        wrapMode: Text.WordWrap
        color: ThemeManager.primary
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeMd
    }
    Rectangle {
        visible: root.pendingAction !== ""
        Layout.fillWidth: true
        implicitHeight: confirmation.implicitHeight + 32
        radius: 18
        color: ThemeManager.surfaceContainerHigh
        ColumnLayout {
            id: confirmation
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
            spacing: 12
            Text {
                Layout.fillWidth: true
                text: (root.pendingAction === "restore" ? I18n.tr("Restore this recovery point? System changes made after it will be reverted. Save your work first; a restart will be needed.") : I18n.tr("Delete this recovery point? This cannot be undone.")) + "\n" + root.pendingName.replace("_", " ")
                wrapMode: Text.WordWrap
                color: ThemeManager.onSurface
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeSm
            }
            RowLayout {
                RecoveryButton { label: I18n.tr("Cancel"); onClicked: root.pendingAction = "" }
                RecoveryButton {
                    label: root.pendingAction === "restore" ? I18n.tr("Restore system") : I18n.tr("Delete")
                    onClicked: { RecoveryService.perform(root.pendingAction, root.pendingName, true); root.pendingAction = "" }
                }
            }
        }
    }
    Repeater {
        model: RecoveryService.data.snapshots || []
        Rectangle {
            required property var modelData
            Layout.fillWidth: true
            implicitHeight: snapshotRow.implicitHeight + 28
            radius: 14
            color: ThemeManager.surfaceContainerLow
            RowLayout {
                id: snapshotRow
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
                spacing: 12
                ColumnLayout {
                    Layout.fillWidth: true
                    Text {
                        text: Qt.formatDateTime(new Date(modelData.created * 1000), "dd MMM yyyy · hh:mm")
                        color: ThemeManager.onSurface
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: ThemeManager.fontSizeMd
                    }
                    Text {
                        text: modelData.description === "Nodalix automatic" ? I18n.tr("Automatic") : modelData.description === "Nodalix manual" ? I18n.tr("Manual") : modelData.description
                        color: ThemeManager.onSurfaceVariant
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: ThemeManager.fontSizeSm
                    }
                }
                RecoveryButton { label: I18n.tr("Restore…"); enabled: modelData.restorable && !RecoveryService.data.rebootRequired; onClicked: { root.pendingName = modelData.name; root.pendingAction = "restore" } }
                RecoveryButton { label: I18n.tr("Delete"); enabled: !RecoveryService.data.rebootRequired; onClicked: { root.pendingName = modelData.name; root.pendingAction = "delete" } }
            }
        }
    }
    Text {
        visible: (RecoveryService.data.snapshots || []).length === 0 && !RecoveryService.busy
        text: I18n.tr("No recovery points yet")
        color: ThemeManager.onSurfaceVariant
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeSm
    }
    component RecoveryButton: Rectangle {
        id: button
        property string label
        signal clicked()
        implicitWidth: caption.implicitWidth + 26
        implicitHeight: 36
        radius: 12
        opacity: enabled && !RecoveryService.busy ? 1 : 0.45
        color: hover.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainer
        Text {
            id: caption
            anchors.centerIn: parent
            text: button.label
            color: ThemeManager.onSurface
            font.family: ThemeManager.fontFor(text)
            font.pixelSize: ThemeManager.fontSizeSm
        }
        HoverHandler { id: hover; enabled: button.enabled && !RecoveryService.busy; cursorShape: Qt.PointingHandCursor }
        TapHandler { enabled: button.enabled && !RecoveryService.busy; onTapped: button.clicked() }
    }
}
