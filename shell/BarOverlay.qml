import QtQuick
import "./theme"
import "./services"

// One bar popout inside the per-monitor overlay host. z follows OverlayManager
// so the last opened panel paints above the others in the same window.
Item {
    id: root

    required property string overlayId
    required property string screenName
    required property real panelTop
    required property real maxHeight
    required property real screenWidth
    required property real borderWidth
    property alias source: loader.source
    property alias sourceComponent: loader.sourceComponent
    readonly property alias item: loader.item

    property bool _loaded: false
    readonly property bool open: OverlayManager.isOpen(overlayId, screenName)
    readonly property bool exiting: OverlayManager.isExiting(overlayId, screenName)
    readonly property bool shown: open || exiting

    z: OverlayManager.zIndex(overlayId, screenName)
    visible: shown || opacity > 0
    enabled: open
    clip: true
    y: panelTop
    width: loader.item ? loader.item.implicitWidth : 180
    height: shown ? Math.min(loader.item ? loader.item.implicitHeight : 0, maxHeight) : 0
    x: {
        const w = width
        const ax = OverlayManager.anchorX(overlayId, screenName)
        const half = w / 2
        return Math.min(Math.max(borderWidth, ax - half), screenWidth - w - borderWidth)
    }
    opacity: open ? 1 : 0
    scale: open ? 1 : 0.98
    Behavior on opacity { NumberAnimation { duration: 160 } }
    Behavior on scale {
        NumberAnimation {
            duration: 220
            easing.type: Easing.Bezier
            easing.bezierCurve: [0.05, 0.7, 0.1, 1.0, 1.0, 1.0]
        }
    }

    onShownChanged: if (shown) _loaded = true

    MouseArea {
        anchors.fill: parent
        z: 1
        propagateComposedEvents: true
        onPressed: (mouse) => {
            OverlayManager.bringToFront(overlayId, screenName)
            mouse.accepted = false
        }
    }

    HoverHandler {
        enabled: root.open
        onHoveredChanged: {
            if (hovered || OverlayManager.isActive(overlayId, screenName))
                PopoutService.panelHovered = hovered
        }
    }

    Loader {
        id: loader
        anchors.centerIn: parent
        width: root.width
        height: root.height
        active: root.shown || root._loaded
    }
}
