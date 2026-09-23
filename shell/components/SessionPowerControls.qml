import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io
import "../services"
import "../theme"
import "../widgets/bar"

Item {
    id: root
    property bool previewOnly: false
    property string pending: ""
    property string error: ""
    implicitWidth: 480
    implicitHeight: content.implicitHeight + 28
    function request(action) {
        if (action !== "suspend" && action !== "reboot" && action !== "poweroff") return
        error = ""
        pending = action
    }
    function confirm() {
        if (!pending || operation.running) return
        if (previewOnly) { error = I18n.tr("Preview · no system action performed"); pending = ""; return }
        operation.command = ["systemctl", pending]
        operation.running = true
        pending = ""
    }
    Process {
        id: operation
        onExited: function(code) { if (code !== 0) root.error = I18n.tr("The system could not complete this action. Your session remains locked.") }
    }
    Rectangle { anchors.fill: parent; radius: 24; visible: root.pending !== "" || root.error !== ""; color: Qt.rgba(0.06, 0.07, 0.09, 0.84); border.color: Qt.rgba(1,1,1,0.12) }
    ColumnLayout {
        id: content
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
        spacing: 12
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            visible: root.pending === ""
            spacing: 22
            PowerTile { label: I18n.tr("Suspend"); iconState: "suspend"; onClicked: root.request("suspend") }
            PowerTile { label: I18n.tr("Reboot"); iconState: "reboot"; onClicked: root.request("reboot") }
            PowerTile { label: I18n.tr("Shut down"); iconState: "shutdown"; onClicked: root.request("poweroff") }
        }
        ColumnLayout {
            visible: root.pending !== ""
            Layout.fillWidth: true
            spacing: 12
            Text {
                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
                text: I18n.tr(root.pending === "reboot" ? "Reboot now?" : root.pending === "poweroff" ? "Shut down now?" : "Suspend now?")
                color: "white"; font.family: ThemeManager.fontFor(text); font.pixelSize: 16; font.weight: Font.Medium
            }
            Text {
                visible: root.pending !== "suspend"
                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                text: I18n.tr("Unsaved work in open applications may be lost.")
                color: Qt.rgba(1,1,1,0.65); font.family: ThemeManager.fontFor(text); font.pixelSize: 12
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                ActionButton { text: I18n.tr("Cancel"); glass: true; onClicked: root.pending = "" }
                ActionButton { text: I18n.tr("Confirm"); emphasized: true; destructive: root.pending !== "suspend"; enabled: !operation.running; onClicked: root.confirm() }
            }
        }
        Text { Layout.fillWidth: true; visible: root.error !== ""; text: root.error; color: "#ffb4ab"; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter; font.pixelSize: 12 }
    }
    component PowerTile: AbstractButton {
        id: tile
        required property string label
        required property string iconState
        Layout.preferredWidth: 88
        implicitHeight: 88
        Accessible.name: label
        background: Item {
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                width: 52; height: 52; radius: 26
                color: tile.hovered || tile.activeFocus ? Qt.rgba(1,1,1,0.22) : Qt.rgba(1,1,1,0.09)
                border.width: 1
                border.color: tile.activeFocus ? ThemeManager.primary : Qt.rgba(1,1,1,0.16)
                scale: tile.down ? 0.93 : 1
                Behavior on color { ColorAnimation { duration: 160 } }
                Behavior on scale { NumberAnimation { duration: 120 } }
            }
        }
        contentItem: Item {
            ShellIcon {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 15
                role: "power.action"
                state: tile.iconState
                color: "white"
                iconSize: 22
            }
            Text { anchors.horizontalCenter: parent.horizontalCenter; y: 64; text: tile.label; color: Qt.rgba(1,1,1,0.76); font.family: ThemeManager.fontFor(text); font.pixelSize: 12 }
        }
    }
}
