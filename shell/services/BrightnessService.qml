pragma Singleton
import QtQuick
import Quickshell.Io
import "."

// Discovers every DDC/CI display and keeps an independent brightness value for
// each one. Internal laptop panels fall back to the kernel backlight class.
QtObject {
    id: root

    property int value: 50
    property bool available: false
    property bool detecting: true
    property string backend: ""
    property var monitors: [] // [{ bus, name, value }]
    property var _pendingSets: ({})
    readonly property int monitorCount: monitors.length

    function set(pct) {
        const p = Math.max(1, Math.min(100, Math.round(pct)))
        value = p
        _backlightSet.command = ["brightnessctl", "-q", "-c", "backlight", "s", p + "%"]
        _backlightSet.running = false
        _backlightSet.running = true
    }

    function setMonitor(bus, pct) {
        const p = Math.max(1, Math.min(100, Math.round(pct)))
        const next = Object.assign({}, _pendingSets)
        next["" + bus] = p
        _pendingSets = next
        _setDelay.restart()
    }

    function _flushSet() {
        if (_ddcSet.running) { _setDelay.restart(); return }
        const keys = Object.keys(_pendingSets)
        if (keys.length === 0) return
        const bus = parseInt(keys[0])
        const p = _pendingSets[keys[0]]
        const next = Object.assign({}, _pendingSets)
        delete next[keys[0]]
        _pendingSets = next
        _ddcSet.command = ["ddcutil", "--bus", "" + bus, "setvcp", "10", "" + p]
        _ddcSet.running = true
    }

    function refresh() {
        if (backend === "ddc" && monitors.length > 0) {
            const buses = monitors.map(m => m.bus).join(" ")
            _getDdc.command = ["sh", "-c",
                "for bus in " + buses + "; do ddcutil --bus \"$bus\" getvcp 10 --brief 2>/dev/null | awk -v b=$bus '{ if ($5 > 0) print b \"|\" int(100 * $4 / $5) }'; done"]
            _getDdc.running = false
            _getDdc.running = true
        } else if (backend === "backlight") {
            _getBacklight.running = false
            _getBacklight.running = true
        } else {
            _detect.running = false
            _detect.running = true
        }
    }

    property Process _ddcSet: Process {
        running: false
        onExited: if (Object.keys(root._pendingSets).length > 0) root._setDelay.restart()
    }
    property Process _backlightSet: Process { running: false }
    property Timer _setDelay: Timer {
        interval: 160
        repeat: false
        onTriggered: root._flushSet()
    }

    property Process _detect: Process {
        command: ["sh", "-c",
            "ddcutil detect --brief 2>/dev/null | awk '/I2C bus:/ { sub(/^.*\\/dev\\/i2c-/, \"\"); bus=$0 } /Monitor:/ { sub(/^[[:space:]]*Monitor:[[:space:]]*/, \"\"); sub(/:$/, \"\"); print bus \"|\" $0 }'"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const found = []
                for (const line of text.trim().split("\n")) {
                    const cut = line.indexOf("|")
                    const bus = parseInt(cut >= 0 ? line.slice(0, cut) : "")
                    if (isNaN(bus)) continue
                    let name = line.slice(cut + 1).trim()
                    const nameParts = name.split(":")
                    if (nameParts.length > 1 && nameParts[1].trim() !== "")
                        name = nameParts[1].trim()
                    found.push({ bus: bus, name: name || (I18n.tr("Display") + " " + (found.length + 1)), value: 50 })
                }
                root.monitors = found
                if (found.length > 0) {
                    root.backend = "ddc"
                    root.available = true
                    root.detecting = false
                    root.refresh()
                } else {
                    root.backend = "backlight"
                    root._getBacklight.running = false
                    root._getBacklight.running = true
                }
            }
        }
    }

    property Process _getDdc: Process {
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const values = {}
                for (const line of text.trim().split("\n")) {
                    const parts = line.split("|")
                    const bus = parseInt(parts[0])
                    const v = parseInt(parts[1])
                    if (!isNaN(bus) && !isNaN(v)) values["" + bus] = v
                }
                let total = 0
                let count = 0
                root.monitors = root.monitors.map(m => {
                    const v = values["" + m.bus]
                    if (v !== undefined) { total += v; count++; return { bus: m.bus, name: m.name, value: v } }
                    return m
                })
                root.available = count > 0
                if (count > 0) root.value = Math.round(total / count)
            }
        }
    }

    property Process _getBacklight: Process {
        command: ["sh", "-c",
            "max=$(brightnessctl -c backlight m 2>/dev/null); cur=$(brightnessctl -c backlight g 2>/dev/null); " +
            "[ -n \"$max\" ] && [ \"$max\" -gt 0 ] && echo $((100 * cur / max)) || echo -1"]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const v = parseInt(text)
                root.available = !isNaN(v) && v >= 0
                root.detecting = false
                if (root.available) root.value = v
            }
        }
    }

    property Timer _poll: Timer {
        interval: 15000
        repeat: true
        running: true
        onTriggered: root.refresh()
    }

    Component.onCompleted: refresh()
}
