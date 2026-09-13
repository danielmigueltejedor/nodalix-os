pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root

    property bool busy: false
    property bool failed: false
    property string statusText: ""
    property string _pendingLanguage: ""

    function apply(language) {
        if (busy || (language !== "es" && language !== "en")) return
        _pendingLanguage = language
        busy = true
        failed = false
        statusText = I18n.tr("Waiting for administrator authorization…")
        _system.command = ["pkexec", "/usr/local/libexec/nodalix-set-system-language", language]
        _system.running = true
    }

    property Process _system: Process {
        running: false
        onExited: (code, status) => {
            if (code !== 0) {
                root.busy = false
                root.failed = true
                root.statusText = I18n.tr("System language could not be changed")
                return
            }
            root.statusText = I18n.tr("Synchronizing application dictionaries…")
            root._user.command = [Paths.configDir + "/scripts/nodalix-sync-user-language.sh", root._pendingLanguage]
            root._user.running = true
        }
    }

    property Process _user: Process {
        running: false
        onExited: (code, status) => {
            root.busy = false
            root.failed = code !== 0
            root.statusText = code === 0
                ? I18n.tr("Language applied · reopen applications or sign out to finish")
                : I18n.tr("Application dictionaries could not be synchronized")
        }
    }
}
