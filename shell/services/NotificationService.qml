pragma Singleton
import QtQuick
import QtMultimedia
import Quickshell
import Quickshell.Hyprland
import Quickshell.Services.Notifications
import "."

QtObject {
    id: root

    property bool doNotDisturb: SettingsService.get("notifications.dndDefault", false)
    property bool centerOpen:   false
    property var  centerScreen: null   // null = all screens (toast); screen obj = bell click
    property int  unreadCount:  0
    property int  notifCount:   0      // explicitly maintained — drives reactive bindings
    property bool toastMode:    false  // true = show only latest notification

    // Hover state
    property bool bellHovered:  false
    property bool panelHovered: false

    readonly property var notifications: _server.trackedNotifications  // raw QML list
    property var notifList: []  // JS array copy — safe for arr[i], .length, etc.

    // ── Toast stack ──────────────────────────────────────────────────────────
    // Each new notification adds its own toast with an independent 5 s expiry —
    // toasts stack instead of replacing one another.
    readonly property int _toastTTL: SettingsService.get("notifications.toastMs", 5000)
    readonly property int _toastMax: SettingsService.get("notifications.toastMax", 5)  // cap visible toast stack
    property var _toastEntries: []                // [{ n: notif, exp: ms }]

    function _isLiveCall(notif) {
        // Calls are the only critical notifications emitted by Enlace móvil.
        return notif
            && ("" + (notif.appName ?? "")) === "Enlace móvil"
            && notif.urgency === NotificationUrgency.Critical
    }

    function _isIncomingCall(notif) {
        if (!_isLiveCall(notif)) return false
        const body = "" + (notif.body ?? "")
        return body === "Llamada entrante" || body === "Incoming call"
    }

    property SoundEffect _notificationSound: SoundEffect {
        source: Qt.resolvedUrl("../assets/sounds/notification.wav")
        volume: 0.72
    }

    property SoundEffect _ringSound: SoundEffect {
        source: Qt.resolvedUrl("../assets/sounds/phone-incoming.wav")
        volume: 0.78
        loops: SoundEffect.Infinite
    }

    // A replacement D-Bus notification can update its body in place without
    // causing NotificationServer.onNotification to run again.  Poll only while
    // the ringtone is playing so an incoming → active transition stops the
    // loop immediately even with notification servers that reuse the object.
    property Timer _ringStateWatch: Timer {
        interval: 120
        repeat: true
        running: root._ringSound.playing
        onTriggered: {
            const stillIncoming = root.notifList.some(n => root._isIncomingCall(n))
            if (!stillIncoming) root._ringSound.stop()
        }
    }

    onDoNotDisturbChanged: { if (doNotDisturb) _ringSound.stop() }

    function _toastExpiry(notif) {
        // A phone call must remain actionable until Enlace móvil replaces or
        // closes it. Normal notifications retain the user-configured timeout.
        return _isLiveCall(notif) ? Number.MAX_SAFE_INTEGER : Date.now() + _toastTTL
    }

    readonly property int toastCount: _toastEntries.length
    // Newest-first for display (newest toast on top)
    readonly property var toastNotifs: {
        const a = []
        for (let i = _toastEntries.length - 1; i >= 0; i--) a.push(_toastEntries[i].n)
        return a
    }

    // Prunes expired toasts; pauses while hovering so they don't vanish mid-read.
    property Timer _toastTimer: Timer {
        interval: 250
        repeat:   true
        onTriggered: {
            if (root.bellHovered || root.panelHovered) return
            const now  = Date.now()
            const kept = root._toastEntries.filter(e => e.exp > now)
            if (kept.length !== root._toastEntries.length) root._toastEntries = kept
            if (kept.length === 0) {
                stop()
                if (root.toastMode) root.closeCenter()
            }
        }
    }

    // Hover-away close delay (300 ms) — only active when NOT opened by bell
    property Timer _closeTimer: Timer {
        interval: 300
        repeat:   false
        onTriggered: { if (!root.bellHovered && !root.panelHovered) root.closeCenter() }
    }

    onBellHoveredChanged:  _evalHover()
    onPanelHoveredChanged: _evalHover()

    function _evalHover() {
        if (!centerOpen) return
        if (bellHovered || panelHovered) {
            _closeTimer.stop()
            _toastTimer.stop()
            if (toastMode) toastMode = false   // expand toast → full view
        } else {
            _closeTimer.restart()
        }
    }

    property NotificationServer _server: NotificationServer {
        keepOnReload:     true
        actionsSupported: true
        bodySupported:    true
        imageSupported:   true

        onNotification: (notif) => {
            notif.tracked = true
            // When the app closes or replaces this notification its backing
            // object is invalidated — drop it so it doesn't linger as a blank
            // row in the center. (Our toast expiry does NOT close notifications,
            // so genuinely-received ones still persist until dismissed.)
            notif.closed.connect(() => {
                if (root._isLiveCall(notif)) root._ringSound.stop()
                root._drop(notif)
            })
            // Most servers expose property-change signals on a replacement
            // notification.  Use it for a zero-delay stop; the short watchdog
            // above remains as a compatibility fallback.
            if (root._isLiveCall(notif) && notif.bodyChanged) {
                notif.bodyChanged.connect(() => {
                    if (!root._isIncomingCall(notif)) root._ringSound.stop()
                })
            }
            root.notifList = [...root.notifList, notif]
            root.notifCount++
            if (!root.doNotDisturb) {
                if (root._isIncomingCall(notif)) root._ringSound.play()
                else {
                    if (root._isLiveCall(notif)) root._ringSound.stop()
                    root._notificationSound.play()
                }
                root.unreadCount++
                // Suppress toast only when the full center is open (bell popout, or
                // inline non-toast center). Otherwise add to the toast stack.
                const popoutOpen     = PopoutService.currentName === "notif"
                const fullCenterOpen = root.centerOpen && !root.toastMode
                if (!popoutOpen && !fullCenterOpen) {
                    let q = [...root._toastEntries, { n: notif, exp: root._toastExpiry(notif) }]
                    if (q.length > root._toastMax) q = q.slice(q.length - root._toastMax)
                    root._toastEntries = q
                    root.toastMode    = true
                    root.centerScreen = null   // show on all screens
                    root.centerOpen   = true
                    root._toastTimer.restart()
                }
            }
        }
    }

    // Emit a shell-local notification (no D-Bus / notify-send). Used for internal
    // alerts like battery state. Builds a plain object shaped like a tracked
    // notification so the toast + center render it the same way.
    function notifyLocal(appName, summary, body, icon, actionCommand) {
        const n = {
            appName: appName || "", summary: summary || "", body: body || "",
            appIcon: icon || "", image: "", actions: [], desktopEntry: "", tracked: true,
            localCommand: actionCommand || []
        }
        notifList = [...notifList, n]
        notifCount++
        if (doNotDisturb) return
        _notificationSound.play()
        unreadCount++
        const popoutOpen     = PopoutService.currentName === "notif"
        const fullCenterOpen = centerOpen && !toastMode
        if (popoutOpen || fullCenterOpen) return
        let q = [..._toastEntries, { n: n, exp: Date.now() + _toastTTL }]
        if (q.length > _toastMax) q = q.slice(q.length - _toastMax)
        _toastEntries = q
        toastMode    = true
        centerScreen = null
        centerOpen   = true
        _toastTimer.restart()
    }

    // Bell opened the center via popout — clear unread + dismiss any live toast
    function markRead() {
        unreadCount = 0
    }

    // Click a notification → bring its app forward.
    //  - window on a special (tray-parked) workspace → reveal that workspace
    //  - window elsewhere → focus it (switches to its workspace)
    //  - no window found → launch the app from its desktop entry
    function activate(notif) {
        if (!notif) return

        // Shell-local notifications can carry an explicit safe action. This is
        // used by LocalSend to open its receive directory with the user's
        // configured default file manager.
        if (notif.localCommand && notif.localCommand.length > 0) {
            Quickshell.execDetached(notif.localCommand)
            closeCenter()
            return
        }

        // Invoke the notification's "default" action first. Messaging apps
        // (Discord, Telegram, Element, …) register it to jump straight to the
        // channel/conversation the message came from — far better than just
        // raising the window. The window-reveal logic below still runs so a
        // tray-parked (special-workspace) app the action can't surface on its
        // own gets revealed too.
        const acts = notif.actions ?? []
        for (let a = 0; a < acts.length; a++) {
            if (("" + (acts[a].identifier ?? "")) === "default") { acts[a].invoke(); break }
        }

        const cands = []
        if (notif.desktopEntry) cands.push(("" + notif.desktopEntry).toLowerCase())
        if (notif.appName)      cands.push(("" + notif.appName).toLowerCase())

        // Find a matching Hyprland window (class ↔ desktop-entry/app-name).
        const tops = Hyprland.toplevels?.values ?? []
        let match = null
        for (let i = 0; i < tops.length && !match; i++) {
            const o = tops[i].lastIpcObject
            if (!o) continue
            const c = ("" + (o["class"] ?? "")).toLowerCase()
            if (!c) continue
            for (let j = 0; j < cands.length; j++) {
                const k = cands[j]
                if (c === k || c.indexOf(k) >= 0 || k.indexOf(c) >= 0) { match = o; break }
            }
        }

        if (match) {
            const addr = match.address
            const wsName = match.workspace ? ("" + (match.workspace.name ?? "")) : ""
            if (wsName.indexOf("special:") === 0) {
                // Reveal the special workspace (tray-parked app) only if it isn't
                // already shown on some monitor — toggle would otherwise hide it.
                const sw = wsName.substring("special:".length)
                let shown = false
                const mons = Hyprland.monitors?.values ?? []
                for (let m = 0; m < mons.length; m++) {
                    const mo = mons[m].lastIpcObject
                    if (mo && mo.specialWorkspace && ("" + (mo.specialWorkspace.name ?? "")) === wsName) { shown = true; break }
                }
                if (!shown)
                    Hyprland.dispatch('hl.dsp.workspace.toggle_special("' + sw + '")')
                if (addr) Hyprland.dispatch('hl.dsp.focus({window = "address:' + addr + '"})')
            } else if (addr) {
                Hyprland.dispatch('hl.dsp.focus({window = "address:' + addr + '"})')
            }
        } else {
            // No live window — launch from the desktop entry if resolvable.
            let app = null
            if (notif.desktopEntry) app = AppService.byKey("" + notif.desktopEntry) || AppService.byClass("" + notif.desktopEntry)
            if (!app && notif.appName) app = AppService.byClass("" + notif.appName)
            if (app) AppService.launch(app)
        }
        closeCenter()
    }

    function dismiss(notif) {
        if (!notif) return
        const wasPresent = notifList.indexOf(notif) >= 0
        // Remove the row before closing the backing D-Bus notification. Setting
        // tracked=false emits `closed` synchronously on some servers; doing it
        // first used to make _drop() update the count and this function then
        // decrement it a second time, hiding the next notification as well.
        notifList = notifList.filter(n => n !== notif)
        _toastEntries = _toastEntries.filter(e => e.n !== notif)
        notifCount = notifList.length
        if (wasPresent && unreadCount > 0) unreadCount--
        if (unreadCount > notifCount) unreadCount = notifCount
        notif.tracked = false
    }

    // Notification was closed/replaced by the app (not a user dismiss). Remove it
    // from the list/toasts so the center never shows a blanked-out entry.
    function _drop(notif) {
        if (notifList.indexOf(notif) < 0) return
        notifList = notifList.filter(n => n !== notif)
        _toastEntries = _toastEntries.filter(e => e.n !== notif)
        notifCount = notifList.length
        // App closed/replaced this notif — keep the unread badge in sync so it
        // can't outlive the list (stale count over an empty center).
        if (unreadCount > notifCount) unreadCount = notifCount
    }

    function dismissAll() {
        // Detach the UI list first for the same reason as dismiss(): every
        // tracked=false can synchronously emit `closed`.
        const old = notifList
        notifList     = []
        _toastEntries = []
        notifCount    = 0
        unreadCount   = 0
        old.forEach(n => { n.tracked = false })
    }

    function openCenter(screen) {
        centerScreen = screen
        toastMode    = false
        centerOpen   = true
        unreadCount  = 0
        _toastTimer.stop()
        _closeTimer.stop()
    }

    function closeCenter() {
        centerOpen    = false
        toastMode     = false
        bellHovered   = false
        panelHovered  = false
        _toastEntries = []
        _closeTimer.stop()
        _toastTimer.stop()
    }

    function toggleCenter(screen) {
        if (centerOpen && !toastMode && centerScreen?.name === screen?.name)
            closeCenter()
        else
            openCenter(screen)
    }
}
