pragma Singleton
import QtQuick
import Quickshell.Hyprland

// Shared, lightweight gaming/fullscreen detector. Services that need to avoid
// background work while a game is visible consume this singleton instead of
// each maintaining their own Hyprland model and polling loop.
QtObject {
    id: root

    // Names of the outputs whose *currently visible* workspace contains a
    // fullscreen window. Keeping this per monitor prevents a fullscreen game
    // parked on a hidden workspace from pausing the visible wallpaper.
    readonly property var fullscreenMonitors: {
        const monitors = Hyprland.monitors?.values ?? []
        const names = []
        for (let i = 0; i < monitors.length; i++) {
            const monitor = monitors[i]
            const workspace = monitor ? monitor.activeWorkspace : null
            if (monitor && workspace && workspace.hasFullscreen)
                names.push(monitor.name)
        }
        return names
    }
    readonly property string fullscreenMonitorNames: fullscreenMonitors.join(",")
    readonly property bool fullscreenActive: fullscreenMonitors.length > 0

    readonly property bool steamGameActive: {
        const toplevel = Hyprland.activeToplevel
        const ipc = toplevel ? toplevel.lastIpcObject : null
        const appClass = ipc ? ("" + (ipc["class"] ?? "")).toLowerCase() : ""
        return appClass.indexOf("steam_app_") === 0
    }

    readonly property bool active: fullscreenActive || steamGameActive

    function initialize() {
        Hyprland.refreshWorkspaces()
        Hyprland.refreshToplevels()
        console.info("GamingService:", active ? "game/fullscreen active" : "desktop mode")
    }
}
