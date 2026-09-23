pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Resolves filesystem paths for the shell. This file lives in services/, so
// ".." is the config root. Two roots:
//   configDir — the (read-only) checkout: code, presets, scripts, plugin.
//   stateDir  — mutable user state, kept OUTSIDE the checkout under XDG state
//               ($XDG_STATE_HOME/nodalix, default ~/.local/state/nodalix):
//               settings, themes, pins, usage, generated binds.
QtObject {
    id: root
    property var userFolders: ({})
    property FileView _folders: FileView {
        path: root.stateDir + "/folders/paths.json"
        watchChanges: true
        blockLoading: true
        onFileChanged: reload()
        onLoaded: { try { root.userFolders = JSON.parse(text()) } catch (e) {} }
    }

    readonly property string homeDir:
        String(Quickshell.env("HOME") || configDir.replace(/\/\.config\/quickshell$/, "")).replace(/\/+$/, "")

    // User-facing media follows the XDG layout reconciled by Nodalix.
    readonly property string downloadsDir: userFolders.DOWNLOAD || homeDir + (I18n.language === "es" ? "/Descargas" : "/Downloads")
    readonly property string picturesDir: userFolders.PICTURES || homeDir + (I18n.language === "es" ? "/Imágenes" : "/Pictures")
    readonly property string videosDir: userFolders.VIDEOS || homeDir + (I18n.language === "es" ? "/Vídeos" : "/Videos")
    readonly property string nodalixPicturesDir: picturesDir + "/Nodalix"
    readonly property string wallpaperDir: nodalixPicturesDir + "/Fondos"
    readonly property string wallpaperImageDir: wallpaperDir + "/Estáticos"
    readonly property string wallpaperAnimatedDir: wallpaperDir + "/Animados"
    readonly property string systemWallpaperDir: "/usr/share/backgrounds/nodalix"
    readonly property string systemWallpaperImageDir: systemWallpaperDir + "/static"
    readonly property string systemWallpaperAnimatedDir: systemWallpaperDir + "/animated"
    readonly property string screenshotsDir: picturesDir + "/Capturas de pantalla"
    readonly property string recordingsDir: videosDir + "/Grabaciones de pantalla"

    // Package-managed shell source, normally /etc/xdg/quickshell/nodalix.
    readonly property string configDir:
        Qt.resolvedUrl("..").toString().replace(/^file:\/\//, "").replace(/\/+$/, "")
    readonly property string scriptsDir: configDir + "/scripts/hypr"
    function script(name) { return scriptsDir + "/" + name }

    // $XDG_STATE_HOME/nodalix, else ~/.local/state/nodalix — same logic
    // as hypr/quickshell.lua and install.sh so all three agree.
    readonly property string stateDir: {
        let base = String(Quickshell.env("XDG_STATE_HOME") || "").trim().replace(/\/+$/, "")
        if (base === "") {
            base = homeDir + "/.local/state"
        }
        return base + "/nodalix"
    }
    function state(name) { return stateDir + "/" + name }
}
