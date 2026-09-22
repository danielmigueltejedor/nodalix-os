import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Widgets
import "../theme"
import "../services"
import "../widgets/bar"
import "../widgets/launcher"

// Windows 11-style start menu — CONTENT ONLY (no window).
// MainWindow draws the blob surface (merged into the bottom screen border),
// positions/animates it, and owns keyboard focus + the input mask. This item
// holds the search field, pinned/recommended/results grids, keyboard nav and
// the context menu. Phases 9a–9f.
Item {
    id: root

    // Driven by MainWindow's loader: true while open on this screen.
    property bool active: false
    // Docked menus use the full bar width; standalone menus retain six columns.
    property bool fillAvailableColumns: false
    // Search header, spacing, actual grids, and outer padding. The host caps
    // this natural height and the Flickable handles large result collections.
    implicitHeight: 44 + 18 + _content.implicitHeight + 44 + ThemeManager.borderWidth

    onActiveChanged: {
        if (active) {
            _pinnedPage = 0
            LauncherService.query = ""
            Qt.callLater(() => _search.forceActiveFocus())
        }
    }
    Component.onCompleted: if (active) _search.forceActiveFocus()

    // ── Data ──────────────────────────────────────────────────────────────────
    readonly property var _pinnedKeys: PinnedService.pinned
    readonly property var _pinned:
        _pinnedKeys.map(k => AppService.byKey(k)).filter(a => a)
    readonly property var _recommended:
        AppUsageService.recommended(_pinnedKeys, root.fillAvailableColumns ? root._cols : 6)
            .map(k => AppService.byKey(k)).filter(a => a)

    // Pinned pagination (tabs on overflow): cols × 3 rows per page.
    readonly property int _pinPageSize: _cols * 3
    readonly property int _pinnedPages: Math.max(1, Math.ceil(_pinned.length / _pinPageSize))
    property int _pinnedPage: 0
    readonly property int _pinPageOff: _pinnedPage * _pinPageSize
    readonly property var _pinnedPageApps: _pinned.slice(_pinPageOff, _pinPageOff + _pinPageSize)
    Connections {
        target: PinnedService
        function onPinnedChanged() {
            if (root._pinnedPage >= root._pinnedPages) root._pinnedPage = root._pinnedPages - 1
            if (root._pinnedPage < 0) root._pinnedPage = 0
        }
    }

    // ── Search ────────────────────────────────────────────────────────────────
    readonly property string _q: LauncherService.query.trim()
    readonly property bool   _searching: _q.length > 0
    readonly property var _settingEntries: [
        { _nodalixKind: "settings", _nodalixGlyph: "󰀄", page: "user", name: I18n.tr("Settings") + " · " + I18n.tr("User"), keywords: "ajustes settings usuario perfil avatar imagen user profile" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰒃", page: "security", name: I18n.tr("Settings") + " · " + I18n.tr("Security"), keywords: "ajustes settings seguridad bloqueo inactividad contraseña security lock idle" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰚰", page: "updates", name: I18n.tr("Settings") + " · " + I18n.tr("Updates"), keywords: "ajustes settings actualizaciones sistema aplicaciones firmware kernel updates apps" },
        { _nodalixKind: "settings", _nodalixGlyph: "󱁤", page: "tools", name: I18n.tr("Settings") + " · " + I18n.tr("Tools"), keywords: "ajustes settings herramientas acciones tools" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰋊", page: "storage", name: I18n.tr("Settings") + " · " + I18n.tr("Storage"), keywords: "ajustes settings almacenamiento disco aplicaciones desinstalar storage disk apps uninstall" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰉼", page: "appearance", name: I18n.tr("Settings") + " · " + I18n.tr("General appearance"), keywords: "ajustes settings apariencia tema colores idioma fuente tipografía letra layout appearance theme color language font" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰍜", page: "bar", name: I18n.tr("Settings") + " · " + I18n.tr("Bar"), keywords: "ajustes settings barra reloj espacios indicadores bar clock workspaces" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰝚", page: "media", name: I18n.tr("Settings") + " · " + I18n.tr("Media"), keywords: "ajustes settings multimedia reproductor visualizador audio media player cava" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰜬", page: "widgets", name: I18n.tr("Settings") + " · " + I18n.tr("Desktop widgets"), keywords: "ajustes settings widgets escritorio reloj calendario media" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰸉", page: "wallpaper", name: I18n.tr("Settings") + " · " + I18n.tr("Wallpaper"), keywords: "ajustes settings fondo fondos animado wallpaper background live" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰇚", page: "localsend", name: I18n.tr("Settings") + " · LocalSend", keywords: "ajustes settings localsend compartir airdrop enviar recibir favoritos" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰂚", page: "notifications", name: I18n.tr("Settings") + " · " + I18n.tr("Notifications"), keywords: "ajustes settings notificaciones avisos silencio molestar notifications dnd" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰌌", page: "keybindings", name: I18n.tr("Settings") + " · " + I18n.tr("Keybindings"), keywords: "ajustes settings atajos teclado binds teclas keybindings shortcuts" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰖐", page: "weather", name: I18n.tr("Settings") + " · " + I18n.tr("Weather"), keywords: "ajustes settings tiempo clima ubicación grados weather location" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰍡", page: "tray", name: I18n.tr("Settings") + " · " + I18n.tr("Tray"), keywords: "ajustes settings bandeja iconos aplicaciones tray indicators" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰍹", page: "hyprland", name: I18n.tr("Settings") + " · " + I18n.tr("Display settings"), keywords: "ajustes settings pantalla pantallas hdr monitor monitores hyprland entrada ratón teclado animaciones display input" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰏖", page: "dependencies", name: I18n.tr("Settings") + " · " + I18n.tr("Dependencies"), keywords: "ajustes settings dependencias paquetes funciones dependencies packages" },
        { _nodalixKind: "settings", _nodalixGlyph: "󰒓", page: "advanced", name: I18n.tr("Settings") + " · " + I18n.tr("Advanced"), keywords: "ajustes settings avanzado configuración restablecer json advanced reset config" }
    ]
    function _settingsFor(query) {
        const words = query.toLowerCase().split(/\s+/).filter(w => w !== "")
        return _settingEntries.filter(e => {
            const hay = (e.name + " " + e.keywords).toLowerCase()
            return words.every(w => hay.indexOf(w) >= 0)
        }).slice(0, 6)
    }
    function _looksLikeAddress(query) {
        return /^(https?:\/\/|www\.|localhost([:\/]|$))/i.test(query)
            || /^[^\s]+\.[a-z]{2,}([:\/].*)?$/i.test(query)
    }
    function _webUrl(query) {
        if (_looksLikeAddress(query))
            return /^https?:\/\//i.test(query) ? query : "https://" + query
        return "https://www.google.com/search?q=" + encodeURIComponent(query)
    }
    function _webEntry(query) {
        return {
            _nodalixKind: "web",
            _nodalixGlyph: "󰍉",
            query: query,
            name: _looksLikeAddress(query)
                ? I18n.tr("Open web address") + " · " + query
                : I18n.tr("Search the web") + " · " + query,
            icon: ""
        }
    }
    readonly property var _results: {
        if (!_searching) return []
        const web = _webEntry(_q)
        const settings = _settingsFor(_q)
        const apps = AppService.search(_q).slice(0, Math.max(0, 39 - settings.length))
        return _looksLikeAddress(_q)
            ? [web].concat(settings, apps).slice(0, 40)
            : settings.concat(apps, [web]).slice(0, 40)
    }

    function _launch(app) {
        if (!app) return
        if (app._nodalixKind === "settings") {
            LauncherService.hide()
            SettingsUi.category = app.page
            SettingsUi.show()
            return
        }
        if (app._nodalixKind === "web") {
            Quickshell.execDetached(["xdg-open", _webUrl(app.query)])
            LauncherService.hide()
            return
        }
        AppService.launch(app)
        LauncherService.hide()
    }

    // ── Keyboard navigation (2D grid in every section) ────────────────────────
    //   _sec: "none" | "results" | "pinned" | "reco"   _idx: index within section
    property string _sec: "none"
    property int    _idx: -1
    readonly property int _cols: Math.max(1, fillAvailableColumns
        ? Math.floor((width - 38) / 94)
        : Math.min(6, Math.floor((width - 38) / 98)))

    function _resetSel() { _sec = _searching ? "results" : "none"; _idx = -1 }

    function _list(sec) {
        if (sec === "results") return _results
        if (sec === "pinned")  return _pinnedPageApps
        if (sec === "reco")    return _recommended
        return []
    }

    function _navDown() {
        if (_searching) {
            if (_idx < 0) { if (_results.length) _idx = 0; return }
            const ni = _idx + _cols
            if (ni < _results.length) _idx = ni
            return
        }
        if (_sec === "none") { if (_pinnedPageApps.length) { _sec = "pinned"; _idx = 0 } return }
        if (_sec === "pinned") {
            const ni = _idx + _cols
            if (ni < _pinnedPageApps.length) _idx = ni
            else if (_recommended.length) { _sec = "reco"; _idx = Math.min(_idx % _cols, _recommended.length - 1) }
            return
        }
        if (_sec === "reco") {
            const ni = _idx + _cols
            if (ni < _recommended.length) _idx = ni
        }
    }
    function _navUp() {
        if (_searching) {
            const ni = _idx - _cols
            _idx = ni >= 0 ? ni : -1
            return
        }
        if (_sec === "reco") {
            const ni = _idx - _cols
            if (ni >= 0) _idx = ni
            else if (_pinnedPageApps.length) {
                const lastRow = Math.floor((_pinnedPageApps.length - 1) / _cols) * _cols
                _sec = "pinned"; _idx = Math.min(lastRow + (_idx % _cols), _pinnedPageApps.length - 1)
            }
            return
        }
        if (_sec === "pinned") {
            const ni = _idx - _cols
            if (ni >= 0) _idx = ni
            else { _sec = "none"; _idx = -1 }
        }
    }
    function _navLeft() {
        if (_searching) {
            if (_idx > 0) _idx--
            else if (_idx < 0 && _results.length) _idx = 0
            return
        }
        if (_sec === "none") return
        if (_sec === "pinned" && _idx % _cols === 0) {
            if (_pinnedPage > 0) { _pinnedPage--; _idx = Math.min(_idx + _cols - 1, _pinnedPageApps.length - 1) }
            return
        }
        if (_idx > 0) _idx--
    }
    function _navRight() {
        if (_searching) {
            if (_idx < _results.length - 1) _idx = Math.max(_idx + 1, 0)
            return
        }
        if (_sec === "none") return
        const l = _list(_sec)
        if (_sec === "pinned" && (_idx % _cols === _cols - 1 || _idx === l.length - 1)) {
            if (_pinnedPage < _pinnedPages - 1) {
                _pinnedPage++
                _idx = Math.min(_idx - (_idx % _cols), _pinnedPageApps.length - 1)
            }
            return
        }
        if (_idx < l.length - 1) _idx++
    }

    function _activateSel() {
        if (_searching) { _launch(_idx >= 0 ? (_results[_idx] ?? null) : (_results[0] ?? null)); return }
        const l = _list(_sec)
        if (_idx >= 0 && _idx < l.length) _launch(l[_idx])
    }

    // ── Context menu ────────────────────────────────────────────────────────--
    property bool _menuOpen: false
    property var  _menuItems: []
    property int  _menuSel: 0
    property real _menuX: 0
    property real _menuY: 0

    // Drag proxy (rendered at root, above the clipped Flickable)
    property var  _dragApp: null
    property real _dragX: 0
    property real _dragY: 0

    function _openMenuFor(section, app, sx, sy) {
        if (!app) return
        const key = AppService.keyOf(app)
        const items = []
        items.push({ label: app._nodalixKind ? I18n.tr("Open") : I18n.tr("Launch"), danger: false, run: () => root._launch(app) })
        if (app._nodalixKind) {
            _menuItems = items
            _menuSel = 0
            const specialPoint = root.mapFromItem(null, sx, sy)
            _menuX = Math.min(Math.max(0, specialPoint.x), root.width - 230)
            _menuY = Math.min(Math.max(0, specialPoint.y), root.height - 48)
            _menuOpen = true
            return
        }
        if (section === "pinned") {
            items.push({ label: I18n.tr("Unpin"), danger: true, run: () => PinnedService.unpin(key) })
        } else {
            items.push(PinnedService.isPinned(key)
                ? { label: I18n.tr("Unpin"), danger: true, run: () => PinnedService.unpin(key) }
                : { label: I18n.tr("Pin"),   danger: false, run: () => PinnedService.pin(key) })
        }
        if (section === "reco")
            items.push({ label: I18n.tr("Remove from recommended"), danger: true, run: () => AppUsageService.reset(key) })
        _menuItems = items
        _menuSel = 0
        // AppGrid emits scene coords → convert to this item's local space.
        const p = root.mapFromItem(null, sx, sy)
        _menuX = Math.min(Math.max(0, p.x), root.width  - 230)
        _menuY = Math.min(Math.max(0, p.y), root.height - (items.length * 36 + 12))
        _menuOpen = true
    }
    function _closeMenu() { _menuOpen = false; _menuItems = [] }
    function _menuDown() { if (_menuItems.length) _menuSel = (_menuSel + 1) % _menuItems.length }
    function _menuUp()   { if (_menuItems.length) _menuSel = (_menuSel - 1 + _menuItems.length) % _menuItems.length }
    function _menuRun()  {
        const it = _menuItems[_menuSel]
        _closeMenu()
        if (it && it.run) it.run()
    }

    // ── Content ─────────────────────────────────────────────────────────────--
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 22
        anchors.bottomMargin: 22 + ThemeManager.borderWidth   // clear the bottom border strip
        spacing: 18

        // ── Search bar ────────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 44
            radius: 22
            color: ThemeManager.surfaceContainerHigh
            border.width: _search.activeFocus ? 2 : 1
            border.color: _search.activeFocus ? ThemeManager.primary
                                              : ThemeManager.outlineVariant

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 12
                spacing: 10

                ColloidIcon {
                    iconName: "edit-find-symbolic"
                    group: "actions"
                    iconSize: 18
                    color: ThemeManager.onSurfaceVariant
                }
                TextField {
                    id: _search
                    Layout.fillWidth: true
                    background: null
                    color: ThemeManager.onSurface
                    font.family: ThemeManager.fontFor(text)
                    font.pixelSize: 15
                    placeholderText: I18n.tr("Search apps, settings or the web…")
                    placeholderTextColor: ThemeManager.onSurfaceVariant
                    verticalAlignment: TextInput.AlignVCenter
                    text: LauncherService.query
                    onTextChanged: {
                        if (text !== LauncherService.query) LauncherService.query = text
                        root._resetSel()
                    }
                    onAccepted: { if (root._menuOpen) root._menuRun(); else root._activateSel() }
                    Keys.onDownPressed:  (e) => { if (root._menuOpen) root._menuDown(); else root._navDown(); e.accepted = true }
                    Keys.onUpPressed:    (e) => { if (root._menuOpen) root._menuUp();   else root._navUp();   e.accepted = true }
                    Keys.onLeftPressed:  (e) => { if (!root._menuOpen) root._navLeft();  e.accepted = true }
                    Keys.onRightPressed: (e) => { if (!root._menuOpen) root._navRight(); e.accepted = true }
                    Keys.onEscapePressed: {
                        if (root._menuOpen) root._closeMenu()
                        else if (root._searching) LauncherService.query = ""
                        else LauncherService.hide()
                    }
                }
            }
        }

        // ── Content: search results OR pinned + recommended ───────────────────
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: _content.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ColumnLayout {
                id: _content
                width: parent.width
                spacing: 14

                AppGrid {
                    visible: root._searching
                    Layout.fillWidth: true
                    title: I18n.tr("Results")
                    apps: root._results
                    columns: root._cols
                    tileW: Math.max(32, (root.width - 44 - 6 * (root._cols - 1)) / root._cols)
                    selectedIndex: root._sec === "results" ? root._idx : -1
                    onActivated: app => root._launch(app)
                    onContextRequested: (app, mx, my) => root._openMenuFor("results", app, mx, my)
                }

                AppGrid {
                    visible: !root._searching
                    Layout.fillWidth: true
                    title: I18n.tr("Pinned")
                    apps: root._pinnedPageApps
                    columns: root._cols
                    tileW: Math.max(32, (root.width - 44 - 6 * (root._cols - 1)) / root._cols)
                    selectedIndex: root._sec === "pinned" ? root._idx : -1
                    draggable: true
                    onActivated: app => root._launch(app)
                    // Reorder by key (display list is filtered/paginated, so raw
                    // indices would desync from PinnedService's key array).
                    onReordered: (from, to) => {
                        const movedApp = root._pinnedPageApps[from]
                        if (!movedApp) return
                        const tgtIdx    = root._pinPageOff + to
                        const beforeApp = tgtIdx < root._pinned.length ? root._pinned[tgtIdx] : null
                        PinnedService.moveBefore(AppService.keyOf(movedApp),
                                                 beforeApp ? AppService.keyOf(beforeApp) : "")
                    }
                    onContextRequested: (app, mx, my) => root._openMenuFor("pinned", app, mx, my)
                    onDragMove: (app, wx, wy) => {
                        root._dragApp = app
                        const p = root.mapFromItem(null, wx, wy)
                        root._dragX = p.x - 46
                        root._dragY = p.y - 46
                    }
                    onDragEnd: root._dragApp = null
                }

                RowLayout {
                    visible: !root._searching && root._pinnedPages > 1
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8
                    Repeater {
                        model: root._pinnedPages
                        delegate: Rectangle {
                            required property int index
                            implicitWidth: index === root._pinnedPage ? 22 : 8
                            implicitHeight: 8
                            radius: 4
                            color: index === root._pinnedPage ? ThemeManager.primary
                                                              : ThemeManager.outlineVariant
                            Behavior on implicitWidth { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
                            TapHandler { onTapped: { root._pinnedPage = index; root._idx = 0 } }
                        }
                    }
                }

                AppGrid {
                    visible: !root._searching && root._recommended.length > 0
                    Layout.fillWidth: true
                    title: I18n.tr("Recommended")
                    apps: root._recommended
                    columns: root._cols
                    tileW: Math.max(32, (root.width - 44 - 6 * (root._cols - 1)) / root._cols)
                    selectedIndex: root._sec === "reco" ? root._idx : -1
                    onActivated: app => root._launch(app)
                    onContextRequested: (app, mx, my) => root._openMenuFor("reco", app, mx, my)
                }

                Item {
                    visible: !root._searching && root._recommended.length === 0
                    Layout.fillWidth: true
                    implicitHeight: 40
                    Text {
                        anchors.centerIn: parent
                        text: I18n.tr("Recommended apps appear here as you use them")
                        color: ThemeManager.onSurfaceVariant
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: ThemeManager.fontSizeSm
                    }
                }
            }
        }
    }

    // ── Drag proxy (above the clipped Flickable, follows the cursor) ──────────
    Rectangle {
        visible: root._dragApp !== null
        x: root._dragX
        y: root._dragY
        width: 92; height: 92
        radius: ThemeManager.chipRadius
        color: ThemeManager.surfaceContainerHigh
        border.width: 2
        border.color: ThemeManager.primary
        opacity: 0.92
        z: 300
        IconImage {
            anchors.centerIn: parent
            implicitSize: 40
            source: root._dragApp ? AppService.iconFor(root._dragApp) : ""
        }
    }

    // ── Context menu overlay (local coords) ───────────────────────────────────
    MouseArea {
        anchors.fill: parent
        visible: root._menuOpen
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: root._closeMenu()
    }
    Rectangle {
        visible: root._menuOpen
        x: root._menuX
        y: root._menuY
        width: 220
        implicitHeight: _menuCol.implicitHeight + 8
        height: implicitHeight
        radius: ThemeManager.chipRadius
        color: ThemeManager.surfaceContainerHigh
        border.width: 1
        border.color: ThemeManager.outlineVariant
        z: 200

        Column {
            id: _menuCol
            width: parent.width
            anchors.verticalCenter: parent.verticalCenter
            Repeater {
                model: root._menuItems
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: 36
                    radius: ThemeManager.chipRadius
                    readonly property bool _sel: root._menuSel === index
                    color: (_sel || _mh.hovered) ? ThemeManager.secondaryContainer : "transparent"
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: 14
                        text: modelData.label
                        color: modelData.danger ? ThemeManager.error : ThemeManager.onSurface
                        font.family: ThemeManager.fontFor(text)
                        font.pixelSize: ThemeManager.fontSizeMd
                    }
                    HoverHandler { id: _mh; onHoveredChanged: if (hovered) root._menuSel = index }
                    TapHandler { onTapped: root._menuRun() }
                }
            }
        }
    }
}
