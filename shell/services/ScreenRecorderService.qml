pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    id: root

    property bool recording: false
    property bool paused: false
    property bool stopping: false
    property bool error: false
    property string message: ""
    property int elapsedSeconds: 0
    property string currentRecordingPath: ""
    property string _fullScreenshotPath: ""
    property string _regionScreenshotPath: ""
    property string _selectedGeometry: ""

    readonly property bool starting: _selectionDelay.running || _selector.running || (_record.running && !recording)
    readonly property string elapsedText: {
        const hours = Math.floor(elapsedSeconds / 3600)
        const minutes = Math.floor((elapsedSeconds % 3600) / 60)
        const seconds = elapsedSeconds % 60
        const mm = String(minutes).padStart(2, "0")
        const ss = String(seconds).padStart(2, "0")
        return hours > 0 ? String(hours).padStart(2, "0") + ":" + mm + ":" + ss
                         : mm + ":" + ss
    }

    function _showSavedNotification(kind, directory) {
        const screenshot = kind === "screenshot"
        NotificationService.notifyLocal(
            "Nodalix",
            I18n.tr(screenshot ? "Screenshot saved" : "Recording saved"),
            I18n.tr(screenshot ? "Click to open the screenshots folder"
                               : "Click to open the recordings folder"),
            screenshot ? "camera-photo" : "media-record",
            ["xdg-open", directory]
        )
    }

    function screenshotFull() {
        if (_fullScreenshot.running) return
        error = false
        _fullScreenshotPath = ""
        _fullScreenshot.command = ["sh", "-c",
            "d=\"$1\"; mkdir -p \"$d\"; f=\"$d/$(date +%Y-%m-%d_%H-%M-%S).png\"; grim \"$f\" || exit $?; if command -v wl-copy >/dev/null; then wl-copy < \"$f\" || true; fi; printf '%s\\n' \"$f\"",
            "sh", Paths.screenshotsDir]
        _fullScreenshot.running = true
    }
    function screenshotRegion() {
        if (_regionScreenshot.running) return
        error = false
        _regionScreenshotPath = ""
        _regionScreenshot.command = ["sh", "-c",
            "d=\"$1\"; mkdir -p \"$d\"; g=$(slurp </dev/null) || exit 130; f=\"$d/$(date +%Y-%m-%d_%H-%M-%S).png\"; grim -g \"$g\" \"$f\" || exit $?; if command -v wl-copy >/dev/null; then wl-copy < \"$f\" || true; fi; printf '%s\\n' \"$f\"",
            "sh", Paths.screenshotsDir]
        _regionScreenshot.running = true
    }
    function startRegion() {
        if (_selectionDelay.running || _selector.running || _record.running) return
        error = false
        stopping = false
        elapsedSeconds = 0
        currentRecordingPath = ""
        _selectedGeometry = ""
        message = I18n.tr("Drag to select the region to record")
        // Give the dashboard time to release its layer-surface input before
        // slurp asks Hyprland for the pointer grab.
        _selectionDelay.restart()
    }

    function cancelSelection() {
        _selectionDelay.stop()
        _selectionTimeout.stop()
        _selectedGeometry = ""
        if (_selector.running) _selector.signal(15)
        message = ""
    }

    function _newRecordingPath() {
        const now = new Date()
        const pad = value => String(value).padStart(2, "0")
        return Paths.recordingsDir + "/" + now.getFullYear() + "-" + pad(now.getMonth() + 1)
            + "-" + pad(now.getDate()) + "_" + pad(now.getHours()) + "-"
            + pad(now.getMinutes()) + "-" + pad(now.getSeconds()) + ".mp4"
    }

    function _beginRecording(geometry) {
        currentRecordingPath = _newRecordingPath()
        _record.command = ["sh", "-c",
            "mkdir -p \"$1\"; exec wf-recorder -g \"$2\" -f \"$3\"",
            "sh", Paths.recordingsDir, geometry, currentRecordingPath]
        _record.running = true
    }
    function togglePause() {
        if (!recording || stopping || !_record.running) return
        // Signal only the recorder started by this service. A global pkill could
        // affect another recorder and leave this card out of sync.
        _record.signal(10) // SIGUSR1: wf-recorder pause/resume
        paused = !paused
        message = paused ? I18n.tr("Recording paused") : I18n.tr("Recording in progress")
    }
    function stop() {
        if (!recording || stopping) return
        stopping = true
        paused = false
        message = I18n.tr("Finishing recording…")
        if (_record.running) {
            _record.signal(2) // SIGINT: lets wf-recorder flush and close the file
            _stopWatchdog.restart()
        } else {
            _resetRecordingState()
        }
    }

    function _resetRecordingState() {
        recording = false
        paused = false
        stopping = false
        elapsedSeconds = 0
        _stopWatchdog.stop()
        _forceStopWatchdog.stop()
    }

    property Process _selector: Process {
        running: false
        onStarted: root._selectionTimeout.restart()
        stdout: StdioCollector {
            onStreamFinished: root._selectedGeometry = text.trim()
        }
        onExited: function(code) {
            root._selectionTimeout.stop()
            const geometry = root._selectedGeometry
            root._selectedGeometry = ""
            if (code === 0 && geometry !== "") {
                root._beginRecording(geometry)
                return
            }
            root.error = code === 127
            root.message = code === 127 ? I18n.tr("wf-recorder is not installed") : ""
            if (root.message !== "") root._clear.restart()
        }
    }
    property Timer _selectionDelay: Timer {
        interval: 350
        onTriggered: {
            root._selector.command = ["sh", "-c",
                "command -v wf-recorder >/dev/null || exit 127; exec slurp -d -b '#00000066' -c '#d1bcfdff' -s '#d1bcfd33' </dev/null"]
            root._selector.running = true
        }
    }
    property Timer _selectionTimeout: Timer {
        interval: 60000
        onTriggered: {
            if (root._selector.running) root._selector.signal(15)
            root._selectedGeometry = ""
            root.message = ""
        }
    }
    property Process _record: Process {
        running: false
        onStarted: {
            root.recording = true
            root.elapsedSeconds = 0
            root.paused = false
            root.stopping = false
            root.message = I18n.tr("Recording in progress")
        }
        onExited: function(code) {
            const savedPath = root.currentRecordingPath
            root._resetRecordingState()
            root.error = code === 127
            root.message = code === 127 ? I18n.tr("wf-recorder is not installed")
                                      : (code === 130 ? ""
                                          : (code === 0 ? I18n.tr("Recording saved")
                                                        : I18n.tr("Recording failed")))
            if (code === 0 && savedPath !== "")
                root._showSavedNotification("recording", Paths.recordingsDir)
            root.currentRecordingPath = ""
            _clear.restart()
        }
    }
    property Process _fullScreenshot: Process {
        running: false
        stdout: StdioCollector { onStreamFinished: root._fullScreenshotPath = text.trim() }
        onExited: function(code) {
            if (code === 0 && root._fullScreenshotPath !== "") {
                root._showSavedNotification("screenshot", Paths.screenshotsDir)
            } else if (code !== 0) {
                root.error = true
                root.message = I18n.tr("Screenshot failed")
                root._clear.restart()
            }
        }
    }
    property Process _regionScreenshot: Process {
        running: false
        stdout: StdioCollector { onStreamFinished: root._regionScreenshotPath = text.trim() }
        onExited: function(code) {
            if (code === 0 && root._regionScreenshotPath !== "") {
                root._showSavedNotification("screenshot", Paths.screenshotsDir)
            } else if (code !== 130) {
                root.error = true
                root.message = I18n.tr("Screenshot failed")
                root._clear.restart()
            }
        }
    }
    property Timer _elapsedTimer: Timer {
        interval: 1000
        repeat: true
        running: root.recording && !root.paused && !root.stopping
        onTriggered: root.elapsedSeconds++
    }
    property Timer _stopWatchdog: Timer {
        interval: 3000
        onTriggered: {
            if (!root._record.running) {
                root._resetRecordingState()
                return
            }
            // A wedged encoder must never leave the desktop permanently in the
            // recording state. Escalate gently before using the final fallback.
            root._record.signal(15) // SIGTERM
            root._forceStopWatchdog.restart()
        }
    }
    property Timer _forceStopWatchdog: Timer {
        interval: 2000
        onTriggered: {
            if (root._record.running)
                root._record.signal(9) // SIGKILL, last resort
            root._resetRecordingState()
            root.error = true
            root.message = I18n.tr("Recording failed")
            root._clear.restart()
        }
    }
    property Timer _clear: Timer {
        interval: 4500
        onTriggered: if (!root.recording) root.message = ""
    }
}
