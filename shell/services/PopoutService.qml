pragma Singleton
import QtQuick
import "."

// Tracks which bar popout is open and where to anchor it.
// Stacking, focus and click-outside are owned by OverlayManager; this
// service keeps hover, pin and per-panel anchor state.
QtObject {
    id: root

    property bool   hasCurrent:   false
    property string currentName:  ""
    property real   anchorX:      0
    property var    anchorScreen: null

    property bool widgetHovered: false
    property bool panelHovered:  false

    // Pinned = stay open regardless of hover (click-to-keep, e.g. notifications)
    property bool pinned: false

    // Set true while a text field inside a hover popout is being edited (the
    // dashboard calendar create form) so the shell layer's focus grab activates —
    // keys reach the field and window focus is restored when it closes.
    property bool textActive: false

    // Arbitrary per-popup payload (e.g. QsMenuHandle for tray menus)
    property var menuHandle: null

    // Last known position per panel name — so reopening the same panel after a
    // brief close (cursor gap between widgets) lands at the original position.
    property var _savedX:      ({})
    property var _savedScreen: ({})

    onWidgetHoveredChanged: _evalHover()
    onPanelHoveredChanged:  _evalHover()

    property Timer _closeTimer: Timer {
        interval: 600   // generous — covers slow cursor movement between adjacent widgets
        repeat:   false
        onTriggered: { if (!root.widgetHovered && !root.panelHovered) root.close() }
    }

    function _evalHover() {
        if (!hasCurrent) return
        if (pinned || widgetHovered || panelHovered)
            _closeTimer.stop()
        else if (OverlayManager.isActive(currentName, anchorScreen?.name ?? ""))
            _closeTimer.restart()
        else
            _closeTimer.stop()
    }

    function _remember(name, x, screen) {
        const savedKey = name + (screen?.name ?? "")
        pinned       = false
        hasCurrent   = true
        currentName  = name
        anchorX      = (savedKey in _savedX) ? _savedX[savedKey] : x
        anchorScreen = screen
        OverlayManager.setAnchor(name, screen?.name ?? "", anchorX)
        _closeTimer.stop()
    }

    function open(name, x, screen) {
        // Same panel already open — don't reposition, just cancel close.
        // Exception: traymenu always repositions (different icon = different position).
        if (hasCurrent && currentName === name &&
                anchorScreen?.name === screen?.name) {
            if (name === "traymenu") {
                anchorX = x
                OverlayManager.setAnchor(name, screen?.name ?? "", x)
            }
            OverlayManager.open(name, screen?.name ?? "", name === "traymenu" ? x : undefined)
            _closeTimer.stop()
            return
        }
        _remember(name, x, screen)
        OverlayManager.open(name, screen?.name ?? "", anchorX)
    }

    function close() {
        if (hasCurrent)
            OverlayManager.close(currentName, anchorScreen?.name ?? "")
        else
            _clear()
    }

    function _clear() {
        if (hasCurrent) {
            const key = currentName + (anchorScreen?.name ?? "")
            const sx = Object.assign({}, _savedX);  sx[key] = anchorX;  _savedX = sx
        }
        hasCurrent    = false
        currentName   = ""
        widgetHovered = false
        panelHovered  = false
        pinned        = false
        textActive    = false
        menuHandle    = null
        _closeTimer.stop()
    }

    function _syncAfterClose(overlayId, screen) {
        if (currentName !== overlayId)
            return
        if (anchorScreen?.name && anchorScreen.name !== screen)
            return
        const remaining = OverlayManager.stack(screen).filter(id => OverlayManager.barIds.indexOf(id) >= 0)
        if (remaining.length) {
            currentName = remaining[remaining.length - 1]
            hasCurrent = true
            anchorX = OverlayManager.anchorX(currentName, screen)
            return
        }
        _clear()
    }

    // Toggle on click (kept for keyboard / alternative trigger)
    function toggle(name, x, screen) {
        const action = OverlayManager.toggle(name, screen?.name ?? "", x)
        if (action === "closed")
            _syncAfterClose(name, screen?.name ?? "")
        else
            _remember(name, x, screen)
    }

    // Click-to-pin: open + keep open ignoring hover. Click again on same → close.
    function togglePin(name, x, screen) {
        if (hasCurrent && currentName === name && pinned &&
                anchorScreen?.name === screen?.name) {
            close()
        } else {
            open(name, x, screen)
            pinned = true
            _closeTimer.stop()
        }
    }

    property Connections _overlayConn: Connections {
        target: OverlayManager
        function onClosed(overlayId, screen) {
            root._syncAfterClose(overlayId, screen)
        }
        function onOpened(overlayId, screen) {
            if (OverlayManager.barIds.indexOf(overlayId) < 0)
                return
            if (root.anchorScreen?.name && root.anchorScreen.name !== screen)
                return
            root.hasCurrent = true
            root.currentName = overlayId
        }
        function onRaised(overlayId, screen) {
            if (OverlayManager.barIds.indexOf(overlayId) < 0)
                return
            if (root.anchorScreen?.name && root.anchorScreen.name !== screen)
                return
            root.currentName = overlayId
            root.hasCurrent = true
        }
    }
}
