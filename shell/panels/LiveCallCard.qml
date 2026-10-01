import QtQuick
import "../theme"
import "../services"

Rectangle {
    id: root

    required property var call

    implicitWidth: 320
    implicitHeight: _content.implicitHeight + ThemeManager.spacing * 2
    radius: ThemeManager.chipRadius
    color: ThemeManager.surfaceContainerHigh
    border.width: 1
    border.color: ThemeManager.primary

    Column {
        id: _content
        x: ThemeManager.spacing
        y: ThemeManager.spacing
        width: parent.width - ThemeManager.spacing * 2
        spacing: 4

        Item {
            width: parent.width
            height: 22

            Text {
                id: _phone
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: "☎"
                color: ThemeManager.primary
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: 18
                font.weight: Font.Bold
            }

            Text {
                anchors {
                    left: _phone.right
                    leftMargin: 6
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
                text: "" + (root.call?.appName ?? "Enlace móvil")
                color: ThemeManager.primary
                font.family: ThemeManager.fontFor(text)
                font.pixelSize: ThemeManager.fontSizeSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
        }

        Text {
            readonly property string value:
                "" + (root.call?.summary ?? "")
            visible: value.length > 0
            width: parent.width
            text: value
            color: ThemeManager.onSurface
            font.family: ThemeManager.fontFor(text)
            font.pixelSize: ThemeManager.fontSizeSm
            font.weight: Font.Medium
            wrapMode: Text.WordWrap
        }

        Text {
            readonly property string value:
                "" + (root.call?.body ?? "")
            visible: value.length > 0
            width: parent.width
            text: value
            color: ThemeManager.onSurfaceVariant
            font.family: ThemeManager.fontFor(text)
            font.pixelSize: ThemeManager.fontSizeSm
            wrapMode: Text.WordWrap
        }

        Row {
            id: _actions
            readonly property var values: root.call?.actions ?? []
            visible: values.length > 0
            width: parent.width
            height: visible ? 34 : 0
            spacing: 8

            Repeater {
                model: _actions.values

                delegate: Rectangle {
                    id: _button
                    required property var modelData
                    readonly property var action: modelData
                    readonly property string actionId:
                        "" + (action?.id ?? "")
                    readonly property bool destructive:
                        actionId === "decline"
                        || actionId === "hangup"

                    width: (_actions.width
                            - Math.max(0, _actions.values.length - 1)
                              * _actions.spacing)
                           / Math.max(1, _actions.values.length)
                    height: 34
                    radius: ThemeManager.chipRadius
                    color: destructive
                        ? Qt.rgba(
                            ThemeManager.error.r,
                            ThemeManager.error.g,
                            ThemeManager.error.b,
                            _hover.hovered ? 0.30 : 0.18
                          )
                        : Qt.rgba(
                            ThemeManager.primary.r,
                            ThemeManager.primary.g,
                            ThemeManager.primary.b,
                            _hover.hovered ? 0.34 : 0.22
                          )

                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 16
                        text: I18n.tr(
                            "" + (_button.action?.text
                                  ?? _button.actionId)
                        )
                        color: _button.destructive
                            ? ThemeManager.error
                            : ThemeManager.primary
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: ThemeManager.fontSizeSm
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignHCenter
                    }

                    HoverHandler {
                        id: _hover
                        cursorShape: Qt.PointingHandCursor
                    }

                    TapHandler {
                        acceptedButtons: Qt.LeftButton
                        onTapped: {
                            NotificationService.invokeCallAction(
                                "" + (root.call?.callPath ?? ""),
                                _button.actionId
                            )
                        }
                    }
                }
            }
        }
    }
}
