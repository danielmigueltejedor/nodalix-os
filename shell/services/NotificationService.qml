pragma Singleton
import QtQuick
import QtMultimedia
import Quickshell
import Quickshell.Io
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

    function _callActionIds(notif) {
        const ids = []
        const actions = notif?.actions ?? []
        for (let i = 0; i < actions.length; i++)
            ids.push("" + (actions[i]?.identifier ?? ""))
        return ids
    }

    function _hint(notif, key, fallback) {
        if (!notif || !notif.hints) return fallback
        const value = notif.hints[key]
        return value === undefined || value === null ? fallback : value
    }

    function _isLiveCall(notif) {
        if (!notif) return false

        // Primary identity: a stable Nodalix-specific hint emitted by
        // Enlace móvil.  This is available as soon as the notification is
        // created and survives replacements throughout the call lifecycle.
        if (Boolean(_hint(notif, "x-nodalix-phone-call", false)))
            return true

        // Compatibility fallback for notifications created by an older daemon.
        if (("" + (notif.appName ?? "")) !== "Enlace móvil")
            return false
        const ids = _callActionIds(notif)
        return ids.indexOf("answer") >= 0
            || ids.indexOf("decline") >= 0
            || ids.indexOf("hangup") >= 0
    }

    function _callPath(notif) {
        return "" + _hint(notif, "x-nodalix-call-path", "")
    }

    function _isIncomingCall(notif) {
        if (!_isLiveCall(notif)) return false
        const ids = _callActionIds(notif)
        return ids.indexOf("answer") >= 0
            || ids.indexOf("decline") >= 0
    }

    function invokeCallAction(callPath, actionId) {
        const path = "" + (callPath ?? "")
        const id = "" + (actionId ?? "")
        if (path === "") return
        if (id !== "answer" && id !== "decline" && id !== "hangup") return

        const method = id === "answer" ? "AnswerCall" : "HangupCall"
        console.info("Phone call action:", id, path)
        Quickshell.execDetached([
            "/usr/bin/busctl", "--user", "call",
            "com.gabriel.iphonebridge",
            "/com/gabriel/iphonebridge",
            "com.gabriel.iphonebridge.Calls1",
            method,
            "s",
            path
        ])
    }

    function invokeNotificationAction(_notif, action) {
        // Ordinary notifications can safely use Quickshell's live
        // NotificationAction. Phone calls never reach this path: their UI uses
        // a plain snapshot and invokeCallAction(), so a replaces_id update cannot
        // invalidate a button by deleting/replacing NotificationAction objects.
        if (action) action.invoke()
    }

    function _snapshotCall(notif) {
        const actions = []
        const source = notif?.actions ?? []
        for (let i = 0; i < source.length; i++) {
            actions.push({
                id: "" + (source[i]?.identifier ?? ""),
                text: "" + (source[i]?.text ?? source[i]?.identifier ?? "")
            })
        }

        return {
            notificationId: Number(notif?.id ?? -1),
            callPath: _callPath(notif),
            appName: "" + (notif?.appName ?? "Enlace móvil"),
            summary: "" + (notif?.summary ?? ""),
            body: "" + (notif?.body ?? ""),
            appIcon: "" + (notif?.appIcon ?? ""),
            image: "" + (notif?.image ?? ""),
            actions: actions
        }
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

    function _pinLiveCallsFirst(items) {
        const calls = []
        const rest = []
        for (let i = 0; i < items.length; i++) {
            const n = items[i]
            if (_isLiveCall(n)) calls.push(n)
            else rest.push(n)
        }
        return calls.concat(rest)
    }

    // Calls are a control surface, not notification history. The authoritative
    // source is Calls1 via nodalix-phone-call-feed; notification snapshots are
    // retained only as a compatibility fallback when Phone Link is unavailable.
    property var liveCallEntries: []
    property var _callFeedEntries: []
    property var _notificationCallEntries: []
    property bool _callFeedAvailable: false
    property var toastNotifs: []
    property var centerNotifs: []
    readonly property int toastCount: liveCallEntries.length + toastNotifs.length

    function _callEntryFromState(call) {
        if (!call) return null
        const path = "" + (call.call_path ?? "")
        const kind = "" + (call.kind ?? "")
        const state = "" + (call.state ?? "")
        if (path === "" || kind === "call_ended" || state === "disconnected")
            return null

        const incoming = kind === "call_incoming"
            || state === "incoming"
            || state === "waiting"
        const outgoing = kind === "call_outgoing"
            || state === "dialing"
            || state === "alerting"

        let body = I18n.tr("Call in progress")
        let actions = [
            { id: "hangup", text: I18n.tr("Hang up") }
        ]
        if (incoming) {
            body = I18n.tr("Incoming call")
            actions = [
                { id: "answer", text: I18n.tr("Answer") },
                { id: "decline", text: I18n.tr("Decline") }
            ]
        } else if (outgoing) {
            body = I18n.tr("Calling…")
        }

        const summary = ""
            + (call.contact_name
               || call.peer_name
               || call.peer_phone
               || I18n.tr("Unknown"))

        return {
            callPath: path,
            appName: "Enlace móvil",
            summary: summary,
            body: body,
            appIcon: "",
            image: "",
            actions: actions
        }
    }

    function _refreshLiveCallEntries() {
        liveCallEntries = _callFeedAvailable
            ? _callFeedEntries
            : _notificationCallEntries
    }

    function _consumeCallFeed(line) {
        if (!line) return
        let payload
        try {
            payload = JSON.parse(line)
        } catch (e) {
            console.warn("Phone call feed JSON error:", e)
            return
        }

        const entries = []
        const calls = payload.calls ?? []
        for (let i = 0; i < calls.length; i++) {
            const entry = _callEntryFromState(calls[i])
            if (entry) entries.push(entry)
        }

        _callFeedAvailable = Boolean(payload.available)
        _callFeedEntries = entries
        _refreshLiveCallEntries()
    }

    property Process _callFeed: Process {
        command: ["/usr/bin/nodalix-phone-call-feed"]
        running: true
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: (line) => root._consumeCallFeed(line)
        }
        onExited: {
            root._callFeedAvailable = false
            root._callFeedEntries = []
            root._refreshLiveCallEntries()
        }
    }

    function _rebuildNotificationViews() {
        const calls = []
        const center = []

        // Calls are excluded from normal notification history. Their snapshots
        // remain only as fallback until the authoritative Calls1 feed is ready.
        for (let i = notifList.length - 1; i >= 0; i--) {
            const n = notifList[i]
            if (_isLiveCall(n))
                calls.push(_snapshotCall(n))
            else
                center.push(n)
        }

        _notificationCallEntries = calls
        _refreshLiveCallEntries()
        centerNotifs = center

        const toast = []
        const seen = []
        for (let i = _toastEntries.length - 1; i >= 0; i--) {
            const n = _toastEntries[i].n
            if (_isLiveCall(n)) continue
            if (seen.indexOf(n) < 0) {
                toast.push(n)
                seen.push(n)
            }
        }
        toastNotifs = toast
    }

    // Prunes expired toasts; pauses while hovering so they don't vanish mid-read.
    property Timer _toastTimer: Timer {
        interval: 250
        repeat:   true
        onTriggered: {
            if (root.bellHovered || root.panelHovered) return
            const now  = Date.now()
            const kept = root._toastEntries.filter(e => e.exp > now)
            if (kept.length !== root._toastEntries.length) {
                root._toastEntries = kept
                root._rebuildNotificationViews()
            }
            if (kept.length === 0) {
                stop()
                if (root.toastMode && root.liveCallEntries.length === 0)
                    root.closeCenter()
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
        extraHints:       [ "x-nodalix-phone-call", "x-nodalix-call-path" ]

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
            // replaces_id mutates this same Notification object in place.
            // Re-snapshot every call-facing property so the pinned call card
            // never holds a deleted NotificationAction or stale call state.
            if (notif.bodyChanged) {
                notif.bodyChanged.connect(() => {
                    if (root._isLiveCall(notif) && !root._isIncomingCall(notif))
                        root._ringSound.stop()
                    root._rebuildNotificationViews()
                })
            }
            if (notif.summaryChanged)
                notif.summaryChanged.connect(root._rebuildNotificationViews)
            if (notif.appIconChanged)
                notif.appIconChanged.connect(root._rebuildNotificationViews)
            if (notif.imageChanged)
                notif.imageChanged.connect(root._rebuildNotificationViews)
            if (notif.actionsChanged)
                notif.actionsChanged.connect(root._rebuildNotificationViews)
            if (notif.hintsChanged)
                notif.hintsChanged.connect(root._rebuildNotificationViews)
            if (notif.appNameChanged)
                notif.appNameChanged.connect(root._rebuildNotificationViews)

            root.notifList = [...root.notifList, notif]
            root.notifCount++
            root._rebuildNotificationViews()
            if (!root.doNotDisturb && !(notif.hints && notif.hints["x-nodalix-silent"])) {
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
                    root._rebuildNotificationViews()
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
        _rebuildNotificationViews()
        if (doNotDisturb) return
        _notificationSound.play()
        unreadCount++
        const popoutOpen     = PopoutService.currentName === "notif"
        const fullCenterOpen = centerOpen && !toastMode
        if (popoutOpen || fullCenterOpen) return
        let q = [..._toastEntries, { n: n, exp: Date.now() + _toastTTL }]
        if (q.length > _toastMax) q = q.slice(q.length - _toastMax)
        _toastEntries = q
        _rebuildNotificationViews()
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
        // A live phone call is a control surface, not a disposable alert.
        // Keep it resident until Enlace móvil closes it when the call ends.
        if (_isLiveCall(notif)) return
        const wasPresent = notifList.indexOf(notif) >= 0
        // Remove the row before closing the backing D-Bus notification. Setting
        // tracked=false emits `closed` synchronously on some servers; doing it
        // first used to make _drop() update the count and this function then
        // decrement it a second time, hiding the next notification as well.
        notifList = notifList.filter(n => n !== notif)
        _toastEntries = _toastEntries.filter(e => e.n !== notif)
        notifCount = notifList.length
        _rebuildNotificationViews()
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
        _rebuildNotificationViews()
        // App closed/replaced this notif — keep the unread badge in sync so it
        // can't outlive the list (stale count over an empty center).
        if (unreadCount > notifCount) unreadCount = notifCount
    }

    function dismissAll() {
        // Keep live-call controls resident. "Clear all" only clears ordinary
        // notifications so Hang up/Answer remains immediately reachable.
        const calls = notifList.filter(n => _isLiveCall(n))
        const old = notifList.filter(n => !_isLiveCall(n))
        notifList = calls
        _toastEntries = _toastEntries.filter(e => _isLiveCall(e.n))
        notifCount = calls.length
        _rebuildNotificationViews()
        unreadCount = Math.min(unreadCount, notifCount)
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
        _rebuildNotificationViews()
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
