pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

QtObject {
    id: root

    readonly property string homeDir: String(Quickshell.env("HOME") || "")
    readonly property string avatarPath: homeDir !== "" ? homeDir + "/.face" : ""
    property int revision: 0
    property bool available: false
    property string _pendingPreset: ""

    readonly property string selectedPreset: SettingsService.get("user.avatarPreset", "")
    readonly property var presets: [
        { id: "luna", name: I18n.tr("Luna"), source: Qt.resolvedUrl("../assets/avatars/luna.png") },
        { id: "nova", name: I18n.tr("Nova"), source: Qt.resolvedUrl("../assets/avatars/nova.png") },
        { id: "wave", name: I18n.tr("Wave"), source: Qt.resolvedUrl("../assets/avatars/wave.png") },
        { id: "prism", name: I18n.tr("Prism"), source: Qt.resolvedUrl("../assets/avatars/prism.png") },
        { id: "orbit-bot", name: I18n.tr("Orbit bot"), source: Qt.resolvedUrl("../assets/avatars/orbit-bot.png") },
        { id: "aeon", name: I18n.tr("Aeon"), source: Qt.resolvedUrl("../assets/avatars/aeon.png") },
        { id: "celeste", name: I18n.tr("Celeste"), source: Qt.resolvedUrl("../assets/avatars/celeste.png") },
        { id: "selene", name: I18n.tr("Selene"), source: Qt.resolvedUrl("../assets/avatars/selene.png") },
        { id: "sora", name: I18n.tr("Sora"), source: Qt.resolvedUrl("../assets/avatars/sora.png") }
    ]

    readonly property string avatarUrl:
        available && avatarPath !== "" ? "file://" + avatarPath + "?v=" + revision : ""

    function _localPath(url) {
        let value = String(url || "")
        if (value.indexOf("file://") === 0) value = value.slice(7)
        try { return decodeURIComponent(value) } catch (e) { return value }
    }

    function setAvatar(url, presetId) {
        const source = _localPath(url)
        if (source === "" || avatarPath === "") return
        _pendingPreset = presetId || ""
        _copy.command = ["sh", "-c", "cp -- \"$1\" \"$2\"", "sh", source, avatarPath]
        _copy.running = false
        _copy.running = true
    }

    function selectPreset(id, source) { setAvatar(source, id) }

    function removeAvatar() {
        if (avatarPath === "") return
        _remove.command = ["rm", "-f", avatarPath]
        _remove.running = false
        _remove.running = true
    }

    function refresh() {
        if (avatarPath === "") { available = false; return }
        _probe.command = ["test", "-f", avatarPath]
        _probe.running = false
        _probe.running = true
    }

    property Process _copy: Process {
        onExited: (code, status) => {
            if (code === 0) {
                root.available = true
                root.revision++
                SettingsService.set("user.avatarPreset", root._pendingPreset)
            }
        }
    }
    property Process _remove: Process {
        onExited: (code, status) => {
            if (code === 0) {
                root.available = false
                root.revision++
                SettingsService.set("user.avatarPreset", "")
            }
        }
    }
    property Process _probe: Process {
        onExited: (code, status) => root.available = (code === 0)
    }

    Component.onCompleted: refresh()
}
