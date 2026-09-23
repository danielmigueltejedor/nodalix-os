import QtQuick
import Quickshell
import Quickshell.Widgets
import Qt5Compat.GraphicalEffects
import "../../theme"
import "../../services"

Item {
    id: root

    // API preferida.
    property string role: ""
    property string state: ""

    // Compatibilidad durante la migración.
    property string iconName: ""
    property string text: ""
    property string group: ""

    property color color: ThemeManager.onSurface

    property font font: Qt.font({ pixelSize: 16 })
    property int iconSize:
        font.pixelSize > 0
            ? font.pixelSize
            : 16

    property int iconPadding: 0

    // -1 significa: usar el perfil central.
    property real visualScale: -1
    property real iconOpacity: -1

    readonly property var iconSpec:
        IconTheme.spec(role, state, iconName)

    readonly property string resolvedName:
        iconSpec.name

    readonly property string iconFile:
        resolvedName !== ""
            ? Quickshell.iconPath(resolvedName, true)
            : ""

    readonly property real effectiveScale:
        visualScale > 0
            ? visualScale
            : iconSpec.scale

    readonly property real effectiveOpacity:
        iconOpacity >= 0
            ? iconOpacity
            : iconSpec.opacity

    readonly property bool iconFailed:
        iconFile === ""
        || image.status === Image.Error

    implicitWidth: iconSize
    implicitHeight: iconSize

    readonly property int renderedSize: Math.max(
        1,
        Math.round(
            (root.iconSize - root.iconPadding * 2)
            * root.effectiveScale
        )
    )

    Image {
        id: image

        anchors.centerIn: parent

        width: root.renderedSize
        height: root.renderedSize

        source: root.iconFile
        fillMode: Image.PreserveAspectFit

        smooth: true
        mipmap: true

        visible: false
    }

    ShaderEffectSource {
        id: imageMask
        anchors.centerIn: parent
        width: image.width
        height: image.height
        sourceItem: image
        hideSource: true
        live: true
        visible: false
    }

    Rectangle {
        id: tintLayer
        anchors.centerIn: parent
        width: image.width
        height: image.height
        color: root.color
        opacity: root.effectiveOpacity
        visible: root.iconFile !== ""

        layer.enabled: visible
        layer.effect: OpacityMask {
            maskSource: imageMask
        }
    }

}
