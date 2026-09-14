pragma Singleton
import QtQuick
import Quickshell.Io
import "."
import "../theme"

// Wallpaper backend (hyprpaper) + Material You theming (matugen), plus favorites
// and a timed rotation. User media lives under the standard Pictures directory:
// ~/Imágenes/Nodalix/Fondos/{Estáticos,Animados}.
//   preview(path) — set live + re-theme, no persistence
//   commit(path)  — preview + persist (settings + hyprpaper.conf)
QtObject {
    id: root

    property var    wallpapers: []   // absolute file paths (local + downloaded)
    property var    animatedWallpapers: []
    property string current:    ""
    property bool   currentAnimated: false
    property string _appliedAnimatedPath: ""

    // The tools rail presents one collection even though static and animated
    // backgrounds use different renderers. Entries carry their type so hover,
    // keyboard preview and commit never try to feed a video to hyprpaper.
    readonly property var railEntries: {
        const paths = (favorites?.length ?? 0) > 0
            ? favorites : wallpapers.concat(animatedWallpapers)
        return paths.map(p => ({ path: "" + p, animated: isAnimatedPath(p) }))
    }
    readonly property var railWallpapers: railEntries.map(e => e.path)

    readonly property string downloadDir: Paths.wallpaperImageDir
    readonly property string animatedDir: Paths.wallpaperAnimatedDir
    readonly property bool animatedAvailable: DependencyService.available("mpvpaper")
    readonly property bool pauseAnimatedFullscreen:
        SettingsService.get("wallpaper.pauseAnimatedFullscreen", true)

    // hyprpaper is the wallpaper backend; without it the whole switcher is off.
    readonly property bool available: DependencyService.available("hyprpaper")
    onAvailableChanged: {
        if (available && current !== "" && !currentAnimated) _restore(current)
    }
    onAnimatedAvailableChanged: {
        // Dependency discovery is asynchronous. At login the saved video can
        // be read before mpvpaper has been detected, so retry as soon as the
        // backend becomes available.
        if (animatedAvailable && current !== "" && currentAnimated)
            _applyAnimated(current)
    }
    onPauseAnimatedFullscreenChanged: {
        if (currentAnimated && animatedAvailable) _syncAnimatedPause()
    }

    property Connections _gamingConnections: Connections {
        target: GamingService
        function onFullscreenMonitorNamesChanged() { root._syncAnimatedPause() }
    }

    // Listing the files needs no backend (only applying does), so don't gate it
    // on `available` — that's resolved asynchronously and would leave an empty
    // list on the first scan.
    function refresh() { _list.running = true }

    function isAnimatedPath(path) {
        return /\.(mp4|webm|mkv|mov)$/i.test("" + path)
    }
    function displayName(path) {
        const value = "" + (path || "")
        const slash = value.lastIndexOf("/")
        return value.slice(slash + 1).replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ")
    }
    function previewEntry(entry) {
        if (!entry || !entry.path) return
        if (entry.animated) previewAnimated(entry.path)
        else preview(entry.path)
    }
    function commitEntry(entry) {
        if (!entry || !entry.path) return
        if (entry.animated) commitAnimated(entry.path)
        else commit(entry.path)
    }

    // ── Favorites ─────────────────────────────────────────────────────────────
    readonly property var favorites: SettingsService.get("wallpaper.favorites", [])
    function isFavorite(p) { return (favorites || []).indexOf(p) >= 0 }
    function toggleFavorite(p) {
        if (!p) return
        const f = (favorites || []).slice()
        const i = f.indexOf(p)
        if (i >= 0) f.splice(i, 1); else f.push(p)
        SettingsService.set("wallpaper.favorites", f)
    }

    // ── Rotation ──────────────────────────────────────────────────────────────
    readonly property bool rotationEnabled:     SettingsService.get("wallpaper.rotation.enabled", false)
    readonly property var  rotationPaths:        SettingsService.get("wallpaper.rotation.paths", [])
    readonly property int  rotationIntervalMin:  SettingsService.get("wallpaper.rotation.intervalMin", 15)
    property int           _rotIndex:            SettingsService.get("wallpaper.rotation.index", 0)

    function setRotationEnabled(b)  { SettingsService.set("wallpaper.rotation.enabled", !!b) }
    function setRotationInterval(m) { SettingsService.set("wallpaper.rotation.intervalMin", Math.max(1, Math.round(m))) }
    function isInRotation(p) { return (rotationPaths || []).indexOf(p) >= 0 }
    function toggleRotation(p) {
        if (!p) return
        const r = (rotationPaths || []).slice()
        const i = r.indexOf(p)
        if (i >= 0) r.splice(i, 1); else r.push(p)
        SettingsService.set("wallpaper.rotation.paths", r)
    }
    function setRotation(paths) { SettingsService.set("wallpaper.rotation.paths", paths || []) }

    // Advance to the next wallpaper in the set (always re-themes via commit).
    function advance() {
        const ps = rotationPaths
        if (!ps || ps.length === 0) return
        _rotIndex = (_rotIndex + 1) % ps.length
        SettingsService.set("wallpaper.rotation.index", _rotIndex)
        commit(ps[_rotIndex])
    }

    // Apply the current rotation entry immediately (on enable / set change).
    function _applyRotationCurrent() {
        const ps = rotationPaths
        if (!ps || ps.length === 0) return
        if (_rotIndex >= ps.length) _rotIndex = 0
        commit(ps[_rotIndex])
    }
    onRotationEnabledChanged: if (rotationEnabled) _applyRotationCurrent()

    property Timer _rotTimer: Timer {
        interval: Math.max(1, root.rotationIntervalMin) * 60000
        repeat:   true
        running:  root.rotationEnabled && (root.rotationPaths?.length ?? 0) > 1
                  && !GamingService.active
        onTriggered: root.advance()
    }

    // ── Apply / persist ───────────────────────────────────────────────────────
    // Live apply (hyprpaper) + matugen theme; no config persistence.
    function preview(path) {
        if (!path) return
        current = path
        currentAnimated = false
        _appliedAnimatedPath = ""
        if (available) {
            _live.command = ["sh", "-c", _liveScript, "sh", path]
            _live.running = true
        }
        ThemeManager.generateWallpaperTheme(path)
    }

    function previewAnimated(path) {
        if (!path) return
        current = path
        currentAnimated = true
        ThemeManager.generateWallpaperTheme(thumbnailFor(path))
        if (animatedAvailable) _applyAnimated(path)
    }

    // Preview + persist (settings + hyprpaper.conf) so it survives a restart.
    function commit(path) {
        if (!path) return
        preview(path)
        SettingsService.set("wallpaper.path", path)
        SettingsService.set("wallpaper.type", "image")
        _syncGreeterWallpaper(path)
        _persistProc.command = ["sh", "-c", _persistScript, "sh", path]
        _persistProc.running = true
    }

    // Re-apply the saved wallpaper image on startup WITHOUT regenerating the
    // theme — the active theme is persisted separately by ThemeManager, so
    // running matugen here would clobber a theme the user kept.
    function _restore(path) {
        if (!path) return
        current = path                 // track it regardless (drives lock-screen sync)
        currentAnimated = false
        _appliedAnimatedPath = ""
        _syncGreeterWallpaper(path)
        if (!available) return         // can't apply without hyprpaper
        _live.command = ["sh", "-c", _liveScript, "sh", path]
        _live.running = true
    }

    // Back-compat alias (mouse click = immediate commit)
    function apply(path) { commit(path) }
    function thumbnailFor(path) {
        const slash = path.lastIndexOf("/")
        return path.slice(0, slash) + "/.thumbs/" + path.slice(slash + 1) + ".jpg"
    }

    // Video wallpapers are rendered natively on the Wayland background layer
    // through one mpvpaper instance per output. Nodalix controls those processes
    // separately so only a fullscreen window on the visible workspace freezes
    // its monitor; fullscreen windows parked on hidden workspaces are ignored.
    function commitAnimated(path, updateTheme) {
        if (!path) return
        if (updateTheme === undefined) updateTheme = true
        current = path
        currentAnimated = true
        SettingsService.set("wallpaper.path", path)
        SettingsService.set("wallpaper.type", "video")
        _syncGreeterWallpaper(path)
        // Matugen needs an image rather than a video. The library scan creates
        // a representative frame for every animation, so use that same frame
        // to derive the shell palette when the user selects an animated wall.
        if (updateTheme) ThemeManager.generateWallpaperTheme(thumbnailFor(path))
        if (animatedAvailable) _applyAnimated(path)
    }

    // Restore a saved animation without rewriting settings or regenerating its
    // palette. Tracking it before the dependency probe finishes is essential:
    // onAnimatedAvailableChanged will then complete the delayed application.
    function _restoreAnimated(path) {
        if (!path) return
        current = path
        currentAnimated = true
        _syncGreeterWallpaper(path)
        if (animatedAvailable) _applyAnimated(path)
    }

    function _applyAnimated(path) {
        if (!path || !animatedAvailable) return
        if (_appliedAnimatedPath === path && _animated.running) return
        _appliedAnimatedPath = path
        _animated.running = false
        _animated.command = ["sh", "-c",
            "pkill -CONT -x mpvpaper 2>/dev/null || true; " +
            "FRAME=$2; " +
            "pgrep -x hyprpaper >/dev/null || { hyprpaper >/dev/null 2>&1 & sleep 0.6; }; " +
            "if [ -f \"$FRAME\" ]; then " +
            "hyprctl hyprpaper preload \"$FRAME\" >/dev/null 2>&1; " +
            "hyprctl hyprpaper wallpaper \",$FRAME\" >/dev/null 2>&1; fi; " +
            "pkill -x mpvpaper 2>/dev/null || true; " +
            "OUTPUTS=$(hyprctl monitors -j | jq -r '.[].name'); " +
            "for OUTPUT in $OUTPUTS; do " +
            "mpvpaper -o 'no-audio loop-file=inf hwdec=auto-safe panscan=1.0' \"$OUTPUT\" \"$1\" & " +
            "done; wait",
            "sh", path, thumbnailFor(path)]
        _animated.running = true
        _pauseSyncTimer.restart()
    }

    // Match each renderer by its exact output argument and stop/continue only
    // the process that belongs to a currently visible fullscreen workspace.
    function _syncAnimatedPause() {
        if (!currentAnimated || !animatedAvailable) return
        _animatedControl.running = false
        _animatedControl.command = ["sh", "-c",
            "PAUSED=,$1,; ENABLED=$2; " +
            "for PID in $(pgrep -x mpvpaper); do " +
            "ARGS=$(tr '\\0' '\\n' < /proc/$PID/cmdline); OUTPUT=''; " +
            "for MONITOR in $(hyprctl monitors -j | jq -r '.[].name'); do " +
            "printf '%s\\n' \"$ARGS\" | grep -Fxq \"$MONITOR\" && { OUTPUT=$MONITOR; break; }; done; " +
            "if [ \"$ENABLED\" = 1 ] && [ -n \"$OUTPUT\" ] && " +
            "printf '%s' \"$PAUSED\" | grep -Fq \",$OUTPUT,\"; then " +
            "kill -STOP $PID; else kill -CONT $PID; fi; done",
            "sh", GamingService.fullscreenMonitorNames,
            pauseAnimatedFullscreen ? "1" : "0"]
        _animatedControl.running = true
    }

    property Timer _pauseSyncTimer: Timer {
        interval: 700
        repeat: false
        onTriggered: root._syncAnimatedPause()
    }

    // greetd runs as an isolated user and cannot read Daniel's home directory.
    // Mirror the selected image or video into its readable cache. Avoid copying
    // large videos again on every shell start when the cached file is identical.
    function _syncGreeterWallpaper(path) {
        if (!path) return
        _greeterSync.command = ["sh", "-c",
            "CACHE=/var/cache/nodalix-greeter/wallpaper; " +
            "POSTER=/var/cache/nodalix-greeter/wallpaper-preview.jpg; " +
            "[ -d /var/cache/nodalix-greeter ] && { " +
            "cmp -s \"$1\" \"$CACHE\" || install -m 0644 \"$1\" \"$CACHE\"; " +
            "case \"${1##*.}\" in " +
            "mp4|MP4|webm|WEBM|mkv|MKV|mov|MOV) " +
            "TMP=${POSTER}.tmp.jpg; " +
            "ffmpeg -y -loglevel error -ss 0.7 -i \"$1\" -frames:v 1 -q:v 2 \"$TMP\" " +
            "&& mv -f \"$TMP\" \"$POSTER\" ;; " +
            "*) rm -f \"$POSTER\" ;; esac; }",
            "sh", path]
        _greeterSync.running = false
        _greeterSync.running = true
    }

    // ── Download (online browser) ─────────────────────────────────────────────
    readonly property bool canDownload: DependencyService.available("curl")
    property string downloadingId: ""   // wallhaven id currently downloading ("" = none)
    property string _dlTarget: ""
    property bool   _dlFavorite: false

    function _extFor(fileType) {
        const t = "" + (fileType || "")
        if (t.indexOf("png") >= 0)  return "png"
        if (t.indexOf("webp") >= 0) return "webp"
        return "jpg"
    }

    // Download a full image to the managed dir, then set it (or favorite it).
    function download(url, id, fileType, favorite) {
        if (!url || !canDownload) return
        const out = downloadDir + "/" + id + "." + _extFor(fileType)
        _dlTarget     = out
        _dlFavorite   = !!favorite
        downloadingId = "" + id
        _dlProc.command = ["sh", "-c",
            "mkdir -p \"$(dirname \"$1\")\"; curl -fsSL --max-time 60 -A 'Mozilla/5.0 quickshell-wallpaper' -o \"$1\" \"$2\"",
            "sh", out, url]
        _dlProc.running = false
        _dlProc.running = true
    }
    property Process _dlProc: Process {
        running: false
        onExited: (code, status) => {
            root.downloadingId = ""
            if (code !== 0) return
            root.refresh()
            if (root._dlFavorite) {
                const f = (root.favorites || []).slice()
                if (f.indexOf(root._dlTarget) < 0) { f.push(root._dlTarget); SettingsService.set("wallpaper.favorites", f) }
            } else {
                root.commit(root._dlTarget)
            }
        }
    }

    readonly property string _liveScript:
        "WP=\"$1\"; " +
        "pkill -CONT -x mpvpaper 2>/dev/null || true; " +
        "pkill -x mpvpaper 2>/dev/null || true; " +
        "pgrep -x hyprpaper >/dev/null || { hyprpaper >/dev/null 2>&1 & sleep 0.6; }; " +
        "hyprctl hyprpaper preload \"$WP\" >/dev/null 2>&1; " +
        "hyprctl hyprpaper wallpaper \",$WP\" >/dev/null 2>&1"

    readonly property string _persistScript:
        "WP=\"$1\"; " +
        "printf 'preload = %s\\nwallpaper = ,%s\\nsplash = false\\n' \"$WP\" \"$WP\" > \"$HOME/.config/hypr/hyprpaper.conf\""

    property Process _live:        Process { running: false }
    property Process _persistProc: Process { running: false }
    property Process _animated:    Process { running: false }
    property Process _animatedControl: Process { running: false }
    property Process _greeterSync: Process { running: false }

    // Scan the organized Nodalix Pictures library. POSIX-sh glob with an
    // existence guard so a non-matching pattern doesn't leak the literal.
    property Process _list: Process {
        command: ["sh", "-c",
            "mkdir -p \"$1\" \"$2/.thumbs\"; " +
            "for d in \"$1\" \"$3\"; do [ -d \"$d\" ] || continue; " +
            "for f in \"$d\"/*.jpg \"$d\"/*.jpeg \"$d\"/*.png \"$d\"/*.webp; do " +
            "[ -e \"$f\" ] && printf 'I|%s\\n' \"$f\"; done; done; " +
            "for d in \"$2\" \"$4\"; do [ -d \"$d\" ] || continue; " +
            "for f in \"$d\"/*.mp4 \"$d\"/*.webm \"$d\"/*.mkv \"$d\"/*.mov; do " +
            "if [ -e \"$f\" ]; then t=\"${f%/*}/.thumbs/$(basename \"$f\").jpg\"; " +
            "if [ ! -e \"$t\" ] && [ -w \"${f%/*}\" ]; then mkdir -p \"${f%/*}/.thumbs\"; " +
            "ffmpeg -loglevel error -ss 1 -i \"$f\" -frames:v 1 -vf 'scale=480:-2' \"$t\"; fi; " +
            "printf 'V|%s\\n' \"$f\"; fi; done; done",
            "sh", root.downloadDir, root.animatedDir,
            Paths.systemWallpaperImageDir, Paths.systemWallpaperAnimatedDir]
        running: false
        stdout: StdioCollector {
            onStreamFinished: {
                const list = [], videos = []
                for (const ln of text.trim().split("\n")) {
                    if (ln.startsWith("I|")) list.push(ln.slice(2))
                    else if (ln.startsWith("V|")) videos.push(ln.slice(2))
                }
                root.wallpapers = list
                root.animatedWallpapers = videos
            }
        }
    }

    Component.onCompleted: {
        refresh()
        const saved = SettingsService.get("wallpaper.path", "")
        if (saved !== "") {
            if (SettingsService.get("wallpaper.type", "image") === "video") _restoreAnimated(saved)
            else _restore(saved)
        }
    }
}
