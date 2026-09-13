pragma Singleton
import QtQuick
import Quickshell.Wayland
import "."

// Locks the current session after a configurable period without user input.
// Quickshell's Wayland idle monitor is compositor-backed, so input on any
// monitor resets the timer and applications can inhibit it while appropriate.
QtObject {
    id: root

    readonly property bool enabled:
        SettingsService.get("security.autoLock.enabled", true)
    readonly property int timeoutMinutes:
        Math.max(1, Number(SettingsService.get("security.autoLock.minutes", "10")))
    readonly property bool pauseFullscreen:
        SettingsService.get("security.autoLock.pauseFullscreen", true)

    readonly property bool fullscreenProtectionActive:
        pauseFullscreen && GamingService.active
    readonly property bool monitoring: _idle.enabled
    property bool _resumeReady: false

    // Avoid a race while Hyprland repopulates its workspace model after a
    // shell reload, and give the user's first keyboard/pointer action time to
    // reset compositor idle state when leaving a controller-driven game.
    property Timer _resumeTimer: Timer {
        interval: 3000
        repeat: false
        onTriggered: root._resumeReady = true
    }
    onFullscreenProtectionActiveChanged: {
        if (fullscreenProtectionActive) {
            _resumeReady = false
            _resumeTimer.stop()
        } else {
            _resumeTimer.restart()
        }
        console.info("AutoLock: fullscreen protection", fullscreenProtectionActive ? "active" : "inactive")
    }

    property IdleMonitor _idle: IdleMonitor {
        enabled: root.enabled && root._resumeReady
                 && !LockService.locked && !root.fullscreenProtectionActive
        timeout: root.timeoutMinutes * 60
        respectInhibitors: true

        onIsIdleChanged: {
            if (isIdle && root.enabled && !LockService.locked)
                LockService.lock()
        }
    }

    // Called once by shell.qml so the singleton is instantiated at startup,
    // even when the Settings window has never been opened.
    function initialize() {
        if (!fullscreenProtectionActive)
            _resumeTimer.restart()
        console.info("AutoLock: enabled", enabled, "timeout", timeoutMinutes,
                     "minutes, fullscreen protection", fullscreenProtectionActive)
    }
}
