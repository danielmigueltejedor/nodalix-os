pragma Singleton
import QtQuick
import Quickshell.Io

// Privacy activity from the Linux media/location stacks. PipeWire reports live
// capture streams; GeoClue exposes whether a client is currently using location.
QtObject {
    id: root

    property bool microphoneActive: false
    property bool cameraActive: false
    property bool locationActive: false
    readonly property bool active: microphoneActive || cameraActive || locationActive

    function refresh() {
        if (_probe.running) return
        _probe.running = true
    }

    property Process _probe: Process {
        command: ["sh", "-c",
            "media=$(pw-dump 2>/dev/null | jq -r '" +
            "[[.[] | select(.info.props[\"media.class\"] == \"Stream/Input/Audio\" and .info.props[\"stream.is-live\"] == true)] | length, " +
            "[.[] | select(.info.props[\"media.class\"] == \"Stream/Input/Video\" and .info.props[\"stream.is-live\"] == true)] | length] | @tsv' 2>/dev/null); " +
            "mic=$(printf '%s' \"$media\" | cut -f1); cam=$(printf '%s' \"$media\" | cut -f2); " +
            "loc=$(busctl --system get-property org.freedesktop.GeoClue2 /org/freedesktop/GeoClue2/Manager org.freedesktop.GeoClue2.Manager InUse 2>/dev/null | awk '{print $2}'); " +
            "[ \"$mic\" -gt 0 ] 2>/dev/null && mic=1 || mic=0; [ \"$cam\" -gt 0 ] 2>/dev/null && cam=1 || cam=0; " +
            "[ \"$loc\" = true ] && loc=1 || loc=0; printf '%s %s %s\\n' \"$mic\" \"$cam\" \"$loc\""]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const fields = text.trim().split(/\s+/)
                root.microphoneActive = fields[0] === "1"
                root.cameraActive = fields[1] === "1"
                root.locationActive = fields[2] === "1"
            }
        }
    }

    property Timer _poll: Timer {
        interval: 2500
        repeat: true
        running: true
        onTriggered: root.refresh()
    }

    Component.onCompleted: refresh()
}
