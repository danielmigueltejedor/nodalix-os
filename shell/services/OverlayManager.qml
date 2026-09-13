pragma Singleton
import QtQuick

// Per-monitor overlay stack. Last opened or raised is active and visually on top.
//
// OPEN != ACTIVE:
//   isOpen   → present in the stack and may keep rendering
//   isActive → last item; keyboard priority and closeTop target
//
// Two families share the stack but not the same exclusivity rules:
//   stacked  (dashboard, launcher, settings, …) can coexist
//   hover    (audio, network, bluetooth, …) replace each other per monitor
//
// Toggle:
//   - shortcut of the top overlay → close it
//   - shortcut of an overlay that is open but not top → bringToFront
//   - shortcut of a closed overlay → open (push)
// Click outside / Escape: closeTop only.
// Stacks never leak across screens.
QtObject {
    id: root

    property bool debug: false
    property int revision: 0
    property var stacks: ({})
    property var exitingMap: ({})
    property var anchors: ({})
    property var exitQueue: []
    property Timer exitTimer: Timer {
        interval: 220
        repeat: false
        onTriggered: {
            const now = Date.now()
            const keep = []
            for (const item of root.exitQueue) {
                if (item.at <= now)
                    root._markExiting(item.id, item.screen, false)
                else
                    keep.push(item)
            }
            root.exitQueue = keep
            if (keep.length)
                restart()
        }
    }

    readonly property var aliases: ({
        "wifi": "network",
        "notifications": "notif",
        "tray": "traymenu",
        "contextmenu": "context-menu"
    })
    readonly property var keyboardIds: ["launcher", "settings", "context-menu"]
    readonly property var stackedIds: [
        "dashboard", "launcher", "settings", "context-menu", "notification-center"
    ]
    readonly property var hoverBarIds: [
        "audio", "power", "powerprofile", "notif",
        "network", "bluetooth", "traymenu", "workspaces"
    ]
    readonly property var barIds: hoverBarIds

    signal opened(string overlayId, string screen)
    signal closed(string overlayId, string screen)
    signal raised(string overlayId, string screen)

    function _id(overlayId) {
        const raw = "" + (overlayId || "")
        return root.aliases[raw] || raw
    }

    function _screen(screen) {
        if (screen === null || screen === undefined)
            return ""
        if (typeof screen === "string")
            return screen
        return screen.name || ""
    }

    function _key(overlayId, screen) {
        return _screen(screen) + "\x1e" + _id(overlayId)
    }

    function _touch() {
        root.revision = root.revision + 1
    }

    function _log(message) {
        if (root.debug)
            console.log("OverlayManager: " + message)
    }

    function isHoverBar(overlayId) {
        return root.hoverBarIds.indexOf(_id(overlayId)) >= 0
    }

    function isStacked(overlayId) {
        return root.stackedIds.indexOf(_id(overlayId)) >= 0
    }

    // Dashboard is stacked (can sit under launcher/settings) and hover-managed
    // (still auto-closes after 600 ms when it is the top overlay and unhovered).
    function isHoverManaged(overlayId) {
        const id = _id(overlayId)
        return isHoverBar(id) || id === "dashboard"
    }

    function stack(screen) {
        const _ = root.revision
        const name = _screen(screen)
        const list = root.stacks[name]
        return list ? list.slice() : []
    }

    function active(screen) {
        const list = stack(screen)
        return list.length ? list[list.length - 1] : ""
    }

    function isOpen(overlayId, screen) {
        const id = _id(overlayId)
        return stack(screen).indexOf(id) >= 0
    }

    function isActive(overlayId, screen) {
        return active(screen) === _id(overlayId)
    }

    function isExiting(overlayId, screen) {
        const _ = root.revision
        return !!root.exitingMap[_key(overlayId, screen)]
    }

    function zIndex(overlayId, screen) {
        const idx = stack(screen).indexOf(_id(overlayId))
        return idx < 0 ? 0 : idx + 1
    }

    function hasStack(screen) {
        return stack(screen).length > 0
    }

    function setAnchor(overlayId, screen, x) {
        const next = Object.assign({}, root.anchors)
        next[_key(overlayId, screen)] = x
        root.anchors = next
        _touch()
    }

    function anchorX(overlayId, screen) {
        const _ = root.revision
        const value = root.anchors[_key(overlayId, screen)]
        return typeof value === "number" ? value : 0
    }

    function _setStack(screen, list) {
        const name = _screen(screen)
        const next = Object.assign({}, root.stacks)
        if (!list.length)
            delete next[name]
        else
            next[name] = list.slice()
        root.stacks = next
        _touch()
        if (root.debug)
            _log("stack [" + stack(name).join(", ") + "] screen " + name)
    }

    function _markExiting(overlayId, screen, on) {
        const key = _key(overlayId, screen)
        const next = Object.assign({}, root.exitingMap)
        if (on)
            next[key] = true
        else
            delete next[key]
        root.exitingMap = next
        _touch()
    }

    function _dismissOtherHover(overlayId, screen) {
        const keep = _id(overlayId)
        const name = _screen(screen)
        for (const other of stack(name)) {
            if (other === keep)
                continue
            if (root.hoverBarIds.indexOf(other) < 0)
                continue
            close(other, name)
        }
    }

    function open(overlayId, screen, x) {
        const id = _id(overlayId)
        const name = _screen(screen)
        if (!id || !name)
            return "noop"
        if (typeof x === "number")
            setAnchor(id, name, x)
        _markExiting(id, name, false)
        let list = stack(name)
        const idx = list.indexOf(id)
        if (idx >= 0)
            list.splice(idx, 1)
        list.push(id)
        _setStack(name, list)
        _dismissOtherHover(id, name)
        _log("open " + id + " screen " + name)
        if (idx >= 0) {
            root.raised(id, name)
            return "raised"
        }
        root.opened(id, name)
        return "opened"
    }

    function bringToFront(overlayId, screen) {
        const id = _id(overlayId)
        const name = _screen(screen)
        if (!isOpen(id, name))
            return "noop"
        return open(id, name)
    }

    function remove(overlayId, screen) {
        const id = _id(overlayId)
        const name = _screen(screen)
        const list = stack(name)
        const idx = list.indexOf(id)
        if (idx < 0)
            return false
        list.splice(idx, 1)
        _setStack(name, list)
        return true
    }

    function close(overlayId, screen) {
        const id = _id(overlayId)
        const name = _screen(screen)
        if (!remove(id, name))
            return "noop"
        _markExiting(id, name, true)
        _log("close " + id + " screen " + name)
        root.closed(id, name)
        const queue = root.exitQueue.slice()
        queue.push({ id: id, screen: name, at: Date.now() + 220 })
        root.exitQueue = queue
        root.exitTimer.restart()
        return "closed"
    }

    function closeTop(screen) {
        const id = active(screen)
        if (!id)
            return "noop"
        return close(id, screen)
    }

    function toggle(overlayId, screen, x) {
        const id = _id(overlayId)
        const name = _screen(screen)
        if (isActive(id, name))
            return close(id, name)
        return open(id, name, x)
    }

    function wantsKeyboard(screen) {
        const top = active(screen)
        return root.keyboardIds.indexOf(top) >= 0
    }

    function acceptsInput(overlayId, screen) {
        return isOpen(overlayId, screen) && !isExiting(overlayId, screen)
    }
}
