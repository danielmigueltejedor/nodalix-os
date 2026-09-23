pragma Singleton
import QtQuick
import Quickshell
import "."
import "../theme"

// State + keyboard navigation for the right-edge tools toolbar.
// The rail is user-defined custom tools (tools.custom = [{name, icon theme name, command}])
// followed by the built-in wallpaper/background picker. Selection 0..N-1 are the
// custom tools; index `wpIndex` (== custom count) is the wallpaper button.
QtObject {
    id: root

    property bool open:       false   // keyboard mode (rail focused via Super+R)
    property bool wpOpen:     false   // wallpaper picker open
    property int  selected:   0       // highlighted rail button (keyboard)
    property int  wpSelected: 0       // highlighted wallpaper (keyboard)

    property string wpFilter: "all"
    property string wpQuery: ""
    property int wpColumns: 3
    readonly property var wpEntries: WallpaperService.wallpapers.concat(WallpaperService.animatedWallpapers)
        .filter((path, index, paths) => paths.indexOf(path) === index)
        .map(path => ({path: path, animated: WallpaperService.isAnimatedPath(path)}))
        .filter(entry => (wpFilter === "all" || (wpFilter === "favorites" ? WallpaperService.isFavorite(entry.path) : wpFilter === "animated" ? entry.animated : !entry.animated))
            && WallpaperService.displayName(entry.path).toLowerCase().includes(wpQuery.toLowerCase().trim()))
    onWpEntriesChanged: wpSelected = Math.min(wpSelected, Math.max(0, wpEntries.length - 1))

    // User tools + the built-in wallpaper button.
    readonly property var  customTools: SettingsService.get("tools.custom", [])

    // System-theme icons offered by Settings -> Tools.
    readonly property var toolIcons: [
        "utilities-terminal-symbolic",
        "folder-symbolic",
        "text-x-generic-symbolic",
        "document-edit-symbolic",
        "drive-harddisk-symbolic",
        "applications-development-symbolic",
        "dialog-warning-symbolic",
        "web-browser-symbolic",
        "audio-x-generic-symbolic",
        "video-x-generic-symbolic",
        "camera-photo-symbolic",
        "media-record-symbolic",
        "image-x-generic-symbolic",
        "system-run-symbolic",
        "preferences-system-symbolic",
        "accessories-calculator-symbolic",
        "x-office-calendar-symbolic",
        "mail-unread-symbolic",
        "mail-message-new-symbolic",
        "folder-download-symbolic",
        "system-search-symbolic",
        "user-trash-symbolic",
        "drive-harddisk-symbolic",
        "bluetooth-symbolic",
        "video-display-symbolic",
        "input-keyboard-symbolic",
        "applications-graphics-symbolic",
        "applications-games-symbolic",
        "accessories-text-editor-symbolic",
        "preferences-system-time-symbolic",
        "network-server-symbolic",
        "security-high-symbolic",
        "user-home-symbolic",
        "applications-utilities-symbolic",
        "starred-symbolic",
        "emblem-favorite-symbolic",
        "view-app-grid-symbolic",
        "application-x-executable-symbolic"
    ]

    // Numeric codepoints keep compatibility without embedding PUA glyphs
    // in the source code.
    readonly property var legacyToolIcons: ({
        "F018D": "utilities-terminal-symbolic",
        "F024B": "folder-symbolic",
        "F0214": "text-x-generic-symbolic",
        "F03EB": "document-edit-symbolic",
        "F01BC": "drive-harddisk-symbolic",
        "F02A2": "applications-development-symbolic",
        "F00E4": "dialog-warning-symbolic",
        "F059F": "web-browser-symbolic",
        "F075A": "audio-x-generic-symbolic",
        "F0567": "video-x-generic-symbolic",
        "F0100": "camera-photo-symbolic",
        "F0EC3": "media-record-symbolic",
        "F02E9": "image-x-generic-symbolic",
        "F0868": "system-run-symbolic",
        "F0493": "preferences-system-symbolic",
        "F0A9A": "accessories-calculator-symbolic",
        "F00ED": "x-office-calendar-symbolic",
        "F01EE": "mail-unread-symbolic",
        "F0B79": "mail-message-new-symbolic",
        "F01DA": "folder-download-symbolic",
        "F0349": "system-search-symbolic",
        "F0A79": "user-trash-symbolic",
        "F02CA": "drive-harddisk-symbolic",
        "F00AF": "bluetooth-symbolic",
        "F0379": "video-display-symbolic",
        "F030C": "input-keyboard-symbolic",
        "F0E0C": "applications-graphics-symbolic",
        "F0297": "applications-games-symbolic",
        "F082E": "accessories-text-editor-symbolic",
        "F0954": "preferences-system-time-symbolic",
        "F015F": "network-server-symbolic",
        "F0483": "security-high-symbolic",
        "F02DC": "user-home-symbolic",
        "F05B7": "applications-utilities-symbolic",
        "F04CE": "starred-symbolic",
        "F08D0": "emblem-favorite-symbolic",
        "F003B": "view-app-grid-symbolic",
        "F0614": "application-x-executable-symbolic"
    })

    function _legacyToolIcon(raw) {
        const value = String(raw ?? "")
        if (value === "")
            return ""

        const cp = value.codePointAt(0)
        if (cp === undefined)
            return ""

        return legacyToolIcons[cp.toString(16).toUpperCase()] ?? ""
    }

    function toolIconName(raw) {
        const value = String(raw ?? "").trim()

        if (value === "")
            return "application-x-executable-symbolic"

        const legacy = _legacyToolIcon(value)
        if (legacy !== "")
            return legacy

        return value
    }

    function migrateLegacyToolIcons() {
        let items = SettingsService.get("tools.custom", []).slice()
        let changed = false

        for (let i = 0; i < items.length; ++i) {
            const oldValue = String(items[i].icon ?? "")
            const migrated = _legacyToolIcon(oldValue)

            if (migrated === "")
                continue

            let entry = Object.assign({}, items[i])
            entry.icon = migrated
            items[i] = entry
            changed = true
        }

        if (changed)
            SettingsService.set("tools.custom", items)
    }

    Component.onCompleted: migrateLegacyToolIcons()

    // Needs matugen (theme generation) and the unified engine (apply the wallpaper).
    readonly property bool wpEnabled:   SettingsService.get("tools.wallpaper", true)
                                        && DependencyService.available("matugen")
                                        && WallpaperService.available
    readonly property int  wpIndex: customTools.length
    readonly property int  count:   customTools.length + (wpEnabled ? 1 : 0)

    function _launch(cmd) {
        if (cmd && ("" + cmd).trim() !== "") Quickshell.execDetached(["sh", "-c", "" + cmd])
    }

    // Wallpaper preview/revert bookkeeping
    property string _origWp:    ""
    property string _origTheme: ""
    property bool   _origAnimated: false
    property bool   _committing: false

    // Keyboard + focus restore handled by MainWindow's HyprlandFocusGrab.
    function toggle() { if (open || wpOpen) close(); else openKbd() }
    function openKbd() { open = true; wpOpen = false; selected = 0 }
    function close()   { open = false; wpOpen = false }

    // ── Wallpaper preview lifecycle ───────────────────────────────────────────
    onWpOpenChanged: {
        if (wpOpen) {
            _origWp     = WallpaperService.current
            _origTheme  = ThemeManager.activeId
            _origAnimated = WallpaperService.currentAnimated
            _committing = false
            const i = root.wpEntries.findIndex(e => e.path === WallpaperService.current)
            wpSelected = i >= 0 ? i : 0
            _previewTimer.restart()
        } else {
            _previewTimer.stop()
            if (!_committing) _revert()
        }
    }
    onWpSelectedChanged: if (wpOpen) _previewTimer.restart()

    property Timer _previewTimer: Timer {
        interval: 350   // debounce so fast arrowing doesn't spam matugen
        onTriggered: {
            const entry = root.wpEntries[root.wpSelected]
            if (entry && (entry.path !== WallpaperService.current || entry.animated !== WallpaperService.currentAnimated))
                WallpaperService.previewEntry(entry)
        }
    }

    function _revert() {
        if (_origWp !== "") WallpaperService.previewEntry({ path: _origWp, animated: _origAnimated })
        if (_origTheme !== "") ThemeManager.setTheme(_origTheme)
    }

    function commitWallpaper(entry) {
        if (!entry) return
        _committing = true
        WallpaperService.commitEntry(entry)
        close()
    }

    // Up / Down move the selection (rail, or wallpaper list when picker open)
    function up() {
        if (wpOpen) moveSelection(-wpColumns)
        else if (count > 0) selected = (selected - 1 + count) % count
    }
    function down() {
        if (wpOpen) moveSelection(wpColumns)
        else if (count > 0) selected = (selected + 1) % count
    }

    function moveSelection(delta) {
        if (wpEntries.length) wpSelected = Math.max(0, Math.min(wpEntries.length - 1, wpSelected + delta))
    }

    // Left / Enter → enter / activate / confirm
    function activate() {
        if (wpOpen) {
            commitWallpaper(root.wpEntries[wpSelected])
            return
        }
        if (selected < customTools.length) {
            _launch(customTools[selected].command)
            close()
        } else if (wpEnabled && selected === wpIndex) {
            wpOpen = true                                   // onWpOpenChanged sets up preview
        }
    }

    // Right / Escape → back out / close (reverts the live preview)
    function back() {
        if (wpOpen) { wpOpen = false; open = true }   // → onWpOpenChanged reverts
        else close()
    }
}
