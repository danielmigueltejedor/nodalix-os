pragma Singleton
import QtQuick
import Quickshell

QtObject {
    id: root

    readonly property string receivedDir: Paths.downloadsDir + "/LocalSend"

    property bool enabled: false
    property bool reachable: false
    property var devices: []
    property var pending: []
    property var favorites: []
    property var transfer: ({ state: "idle", message: "" })
    property string alias: "Nodalix"
    property string lastError: ""
    property var _notifiedPending: ({})
    property string _lastTransferState: "idle"
    property string _lastReceivedEventId: ""

    function _request(method, path, payload, callback) {
        const request = new XMLHttpRequest()
        request.open(method, "http://127.0.0.1:53318" + path)
        request.setRequestHeader("Content-Type", "application/json")
        request.onreadystatechange = function() {
            if (request.readyState !== XMLHttpRequest.DONE) return
            if (callback) callback(request.status, request.responseText)
        }
        request.send(payload === undefined ? null : JSON.stringify(payload))
    }

    function refresh() {
        _request("GET", "/status", undefined, function(status, body) {
            if (status !== 200) {
                root.reachable = false
                root.enabled = false
                root.devices = []
                root.pending = []
                return
            }
            try {
                const value = JSON.parse(body)
                root.reachable = true
                root.enabled = true
                root.alias = value.alias || "Nodalix"
                root.devices = value.devices || []
                root.pending = value.pending || []
                root.favorites = value.favorites || []
                root.transfer = value.transfer || ({ state: "idle", message: "" })
                root._notifyChanges()
                root.lastError = ""
            } catch (error) {
                root.lastError = String(error)
            }
        })
    }

    function _notifyChanges() {
        const seen = Object.assign({}, _notifiedPending)
        for (let i = 0; i < pending.length; i++) {
            const item = pending[i]
            if (!seen[item.id]) {
                seen[item.id] = true
                NotificationService.notifyLocal("LocalSend", I18n.tr("Incoming transfer"),
                    (item.alias || I18n.tr("Unknown")) + " · " + (item.files ? item.files.length : 0) + " " + I18n.tr("files"),
                    "localsend", ["xdg-open", root.receivedDir])
            }
        }
        _notifiedPending = seen
        const state = transfer && transfer.state ? transfer.state : "idle"

        // Incoming transfers can complete between two polling cycles. Using
        // state alone (done -> done) loses the notification, so every received
        // transfer carries a unique eventId from the daemon.
        const receivedEventId = transfer && transfer.direction === "receive"
            ? (transfer.eventId || "")
            : ""

        if (state === "done" &&
            transfer.direction === "receive" &&
            receivedEventId !== "" &&
            receivedEventId !== _lastReceivedEventId) {

            _lastReceivedEventId = receivedEventId

            NotificationService.notifyLocal(
                "LocalSend",
                I18n.tr("Transfer completed"),
                transfer.message || "",
                "localsend",
                ["xdg-open", root.receivedDir]
            )
        }

        // Preserve the old behaviour for outgoing transfers.
        if (state === "done" &&
            transfer.direction !== "receive" &&
            _lastTransferState !== "done") {

            NotificationService.notifyLocal(
                "LocalSend",
                I18n.tr("Transfer completed"),
                transfer.message || "",
                "localsend",
                ["xdg-open", root.receivedDir]
            )
        }

        _lastTransferState = state
    }

    function setEnabled(value) {
        root.enabled = value
        Quickshell.execDetached(["systemctl", "--user", value ? "start" : "stop", "nodalix-localsend.service"])
        _settle.restart()
    }

    function announce() { _request("POST", "/announce", {}) }
    function setAlias(value) { _request("POST", "/alias", { alias: value }, function() { root.refresh() }) }
    function setFavorite(fingerprint, favorite, alias) {
        _request("POST", "/favorite", { fingerprint: fingerprint, favorite: favorite, alias: alias || "" }, function() { root.refresh() })
    }
    function openReceivedFolder() {
        Quickshell.execDetached(["xdg-open", root.receivedDir])
    }
    function accept(id) { _request("POST", "/accept", { id: id }, function() { root.refresh() }) }
    function decline(id) { _request("POST", "/decline", { id: id }, function() { root.refresh() }) }
    function send(fingerprint, paths) {
        if (!fingerprint || !paths || paths.length === 0) return
        root.transfer = ({ state: "preparing", message: "" })
        _request("POST", "/send", { fingerprint: fingerprint, paths: paths }, function(status) {
            if (status !== 202) root.lastError = "LocalSend request failed"
            _settle.restart()
        })
    }

    property Timer _poll: Timer {
        interval: 1500
        repeat: true
        running: true
        onTriggered: root.refresh()
    }
    property Timer _settle: Timer {
        interval: 650
        repeat: false
        onTriggered: root.refresh()
    }

    Component.onCompleted: refresh()
}
