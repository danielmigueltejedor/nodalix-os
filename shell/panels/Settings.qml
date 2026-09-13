import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Dialogs
import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Hyprland
import Quickshell.Io
import Quickshell.Services.SystemTray
import Quickshell.Networking
import Quickshell.Bluetooth
import Quickshell.Services.UPower
import "../theme"
import "../services"

// Settings — centered modal hosted inside MainWindow's overlay stack so it
// paints above/below launcher and other popups on the same monitor.
Item {
	id: root
	required property var modelData
	anchors.fill: parent

	readonly property bool active:
		OverlayManager.isOpen("settings", modelData?.name)

	visible: active || _exiting

	property bool _exiting: false
	onActiveChanged: {
		if (active) {
			_exiting = false
			SystemControlService.refresh()
		} else if (visible) {
			_exiting = true
			_exitTimer.restart()
		}
	}
	Timer { id: _exitTimer; interval: 180; onTriggered: root._exiting = false }

	FileDialog {
		id: _avatarDialog
		title: I18n.tr("Choose profile image")
		fileMode: FileDialog.OpenFile
		nameFilters: [I18n.tr("Images") + " (*.png *.jpg *.jpeg *.webp)"]
		onAccepted: ProfileService.setAvatar(selectedFile)
	}

	// ── Tray config helpers (defaults mirror Tray.qml) ────────────────────────
	readonly property var _trayHiddenDef: ["nm-applet", "blueman", "slimbookintelcontrollerindicator.py",
		"Wayland to X11 Video bridge", "Gestionnaire de paramètres de Manjaro"]
	readonly property var _traySpecialDef: [
		{ match: "spotify", ws: "spotify" }, { match: "youtube", ws: "ytmusic" },
		{ match: "rocket", ws: "rocketchat" }, { match: "vesktop", ws: "vesktop" },
		{ match: "discord", ws: "vesktop" }, { match: "vencord", ws: "vesktop" }]

	function _trayHay(item) { return ((item.id ?? "") + " " + (item.title ?? "") + " " + (item.tooltipTitle ?? "")).toLowerCase() }
	function _trayHidden() { return SettingsService.get("tray.hidden", _trayHiddenDef) }
	function _trayIsHidden(item) { const h = _trayHay(item); return _trayHidden().some(x => h.includes(String(x).toLowerCase())) }
	function _trayToggleHide(item) {
		const h = _trayHay(item)
		let a = _trayHidden().slice()
		if (_trayIsHidden(item)) a = a.filter(x => !h.includes(String(x).toLowerCase()))
		else a.push((item.title && item.title !== "") ? item.title : (item.id ?? ""))
		SettingsService.set("tray.hidden", a)
	}
	function _traySpecial() { return SettingsService.get("tray.specialWs", _traySpecialDef) }
	function _trayWs(item) { const h = _trayHay(item); const e = _traySpecial().find(x => h.includes(String(x.match).toLowerCase())); return e ? e.ws : "" }
	// Most specific identifier: electron apps share id "chrome_status_icon_1",
	// so prefer the (unique) tooltip title, then title, then id.
	function _trayKey(item) {
		const tt = item.tooltipTitle ?? "", t = item.title ?? "", id = item.id ?? ""
		return (tt !== "" ? tt : (t !== "" ? t : id)).toLowerCase()
	}
	function _traySetWs(item, ws) {
		const key = _trayKey(item)
		let m = _traySpecial().filter(x => String(x.match).toLowerCase() !== key)
		if (ws && ws.trim() !== "") m.push({ match: key, ws: ws.trim() })
		SettingsService.set("tray.specialWs", m)
	}

	// Custom (non-SNI) tray entries: [{name, icon, action:"run"|"ws", value}]
	readonly property var _trayCustomList: SettingsService.get("tray.custom", [])
	property int _trayEditIdx: -1   // which entry is expanded for editing (-1 = none)
	function _trayCustom() { return SettingsService.get("tray.custom", []) }
	function _trayCustomAdd() {
		let a = _trayCustom().slice()
		a.push({ name: I18n.tr("App"), icon: "application-x-executable", action: "run", value: "" })
		SettingsService.set("tray.custom", a)
		_trayEditIdx = a.length - 1   // open the new entry for editing
	}
	function _trayCustomSet(i, field, val) {
		let a = _trayCustom().slice()
		if (i < 0 || i >= a.length) return
		let e = Object.assign({}, a[i]); e[field] = val; a[i] = e
		SettingsService.set("tray.custom", a)
	}
	function _trayCustomRemove(i) {
		let a = _trayCustom().slice()
		if (i < 0 || i >= a.length) return
		a.splice(i, 1)
		SettingsService.set("tray.custom", a)
		_trayEditIdx = -1
	}

	// Custom tools: [{name, icon (glyph), command}] — rendered in the rail.
	readonly property var _toolCustomList: SettingsService.get("tools.custom", [])
	property int _toolEditIdx: -1
	function _toolCustom() { return SettingsService.get("tools.custom", []) }
	function _toolCustomAdd() {
		let a = _toolCustom().slice()
		a.push({ name: I18n.tr("Tool"), icon: "󰘔", command: "" })
		SettingsService.set("tools.custom", a)
		_toolEditIdx = a.length - 1
	}
	function _toolCustomSet(i, field, val) {
		let a = _toolCustom().slice()
		if (i < 0 || i >= a.length) return
		let e = Object.assign({}, a[i]); e[field] = val; a[i] = e
		SettingsService.set("tools.custom", a)
	}
	function _toolCustomRemove(i) {
		let a = _toolCustom().slice()
		if (i < 0 || i >= a.length) return
		a.splice(i, 1)
		SettingsService.set("tools.custom", a)
		_toolEditIdx = -1
	}
	// Curated glyph palette (Material Design Icons / nerd font) for the picker.
	readonly property var _toolIcons: [
		"󰆍","󰉋","󰈔","󰏫","󰆼","󰊢","󰃤","󰖟","󰝚","󰕧","󰄀","󰻃","󰋩","󰡨","󰒓","󰪚",
		"󰃭","󰇮","󰭹","󰇚","󰍉","󰩹","󰋊","󰂯","󰍹","󰌌","󰸌","󰊗","󰠮","󰥔","󰅟","󰒃",
		"󰋜","󰖷","󰓎","󰣐","󰀻","󰘔"
	]

	// Theme name-entry flow: "" | "new" | "duplicate" | "rename"
	property string _themeAction: ""
	function _confirmTheme(name) {
		if (!name || name.trim() === "") { _themeAction = ""; return }
		if (_themeAction === "new")            ThemeManager.createTheme(name)
		else if (_themeAction === "duplicate") ThemeManager.duplicateTheme(ThemeManager.activeId, name)
		else if (_themeAction === "rename")    ThemeManager.renameTheme(ThemeManager.activeId, name)
		_themeAction = ""
	}

	readonly property var _cats: [
		{ id: "personal-group", label: I18n.tr("Personal"), icon: "󰀄" },
		{ id: "appearance-group", label: I18n.tr("Appearance"), icon: "󰉼" },
		{ id: "connections-group", label: I18n.tr("Connections"), icon: "󰖩" },
		{ id: "devices-group", label: I18n.tr("Devices"), icon: "󰍹" },
		{ id: "applications-group", label: I18n.tr("Applications"), icon: "󰀻" },
		{ id: "services-group", label: I18n.tr("Services"), icon: "󰒓" },
		{ id: "system-group", label: I18n.tr("System"), icon: "󰍹" }
	]
	readonly property var _groups: ({
		"personal-group": [
			{ id: "user", label: I18n.tr("User"), sub: I18n.tr("Profile, image and personal settings"), icon: "󰀄" },
			{ id: "security", label: I18n.tr("Security"), sub: I18n.tr("Automatic lock and inactivity"), icon: "󰒃" },
			{ id: "date-time", label: I18n.tr("Date and time"), sub: I18n.tr("Time zone and automatic synchronization"), icon: "󰥔" }
		],
		"appearance-group": [
			{ id: "appearance", label: I18n.tr("General appearance"), sub: I18n.tr("Theme, colors, language and layout"), icon: "󰉼" },
			{ id: "bar", label: I18n.tr("Bar"), sub: I18n.tr("Clock, workspaces and indicators"), icon: "󰍜" },
			{ id: "media", label: I18n.tr("Media"), sub: I18n.tr("Player and audio visualizer"), icon: "󰝚" },
			{ id: "widgets", label: I18n.tr("Desktop widgets"), sub: I18n.tr("Visible widgets and positions"), icon: "󰜬" },
			{ id: "wallpaper", label: I18n.tr("Wallpaper"), sub: I18n.tr("Static and animated backgrounds"), icon: "󰸉" }
		],
		"connections-group": [
			{ id: "connectivity", label: I18n.tr("Connectivity"), sub: I18n.tr("Wi-Fi, Bluetooth and VPN"), icon: "󰖩" },
			{ id: "localsend", label: "LocalSend", sub: I18n.tr("Nearby sharing"), icon: "󰇚" }
		],
		"devices-group": [
			{ id: "hyprland", label: I18n.tr("Display settings"), sub: I18n.tr("Displays, HDR, input and animations"), icon: "󰍹" },
			{ id: "sound", label: I18n.tr("Sound"), sub: I18n.tr("Volume, microphone and audio devices"), icon: "󰕾" },
			{ id: "power", label: I18n.tr("Power"), sub: I18n.tr("Performance profile and session controls"), icon: "󰚥" }
		],
		"applications-group": [
			{ id: "default-apps", label: I18n.tr("Default applications"), sub: I18n.tr("Choose which app opens each file type"), icon: "󰏖" },
			{ id: "storage", label: I18n.tr("Storage"), sub: I18n.tr("Disk usage and installed applications"), icon: "󰋊" },
			{ id: "updates", label: I18n.tr("Updates"), sub: I18n.tr("System, applications and firmware"), icon: "󰚰" },
			{ id: "tray", label: I18n.tr("Tray"), sub: I18n.tr("Application indicators"), icon: "󰍡" }
		],
		"services-group": [
			{ id: "notifications", label: I18n.tr("Notifications"), sub: I18n.tr("Alerts and Do Not Disturb"), icon: "󰂚" },
			{ id: "privacy", label: I18n.tr("Privacy"), sub: I18n.tr("Microphone, camera, location and interruptions"), icon: "󰒃" },
			{ id: "weather", label: I18n.tr("Weather"), sub: I18n.tr("Location and units"), icon: "󰖐" }
		],
		"system-group": [
			{ id: "keybindings", label: I18n.tr("Keybindings"), sub: I18n.tr("Keyboard shortcuts"), icon: "󰌌" },
			{ id: "tools", label: I18n.tr("Tools"), sub: I18n.tr("Quick actions and custom tools"), icon: "󱁤" },
			{ id: "dependencies", label: I18n.tr("Dependencies"), sub: I18n.tr("Optional system features"), icon: "󰏖" },
			{ id: "advanced", label: I18n.tr("Advanced"), sub: I18n.tr("Configuration and reset"), icon: "󰒓" },
			{ id: "about", label: I18n.tr("About this system"), sub: I18n.tr("Hardware and operating system information"), icon: "󰋼" }
		]
	})
	function _groupFor(page) {
		if (page === "storage-apps") return "applications-group"
		for (const key of Object.keys(_groups)) if (_groups[key].some(x => x.id === page)) return key
		return page
	}
	function _groupLabel(group) {
		const item = _cats.find(x => x.id === group); return item ? item.label : I18n.tr("Settings")
	}
	function _pageLabel(page) {
		if (page === "storage-apps") return I18n.tr("Installed applications")
		const group = _groupFor(page), list = _groups[group] || []
		const item = list.find(x => x.id === page); return item ? item.label : _groupLabel(group)
	}
	function _goBack() {
		const group = _groupFor(SettingsUi.category)
		if (group !== SettingsUi.category) SettingsUi.category = group
		else SettingsUi.hide()
	}

	// Wallpaper pane: active sub-tab ("local" | "favorites" | "browse")
	property string _wpTab: "local"
	property string _storageSearch: ""

	// Live hardware state shared by the new system settings pages.
	readonly property var _networkDevices: Networking.devices?.values ?? []
	readonly property var _connectedWifi: {
		for (const device of _networkDevices) {
			if (!device || device.type !== DeviceType.Wifi) continue
			const networks = device.networks?.values ?? []
			for (const network of networks) if (network && network.connected) return network
		}
		return null
	}
	readonly property bool _ethernetConnected: {
		for (const device of _networkDevices)
			if (device && device.type === DeviceType.Ethernet && device.connected) return true
		return false
	}
	readonly property var _bluetoothAdapter: Bluetooth.defaultAdapter

	function _defaultAppIndex(id) {
		const apps = AppService.apps
		for (let i = 0; i < apps.length; i++) if (AppService.keyOf(apps[i]) === id) return i
		return -1
	}

	// Hyprland pane: active sub-tab ("display" | "appearance" | "input")
	property string _hlTab: "display"

	// ── Scrim ───────────────────────────────────────────────────────────────--
	Rectangle {
		anchors.fill: parent
		color: ThemeManager.scrim
		opacity: root.active ? 0.4 : 0
		Behavior on opacity { NumberAnimation { duration: 160 } }
		MouseArea { anchors.fill: parent; onClicked: OverlayManager.closeTop(modelData?.name) }
	}

	// ── Card ────────────────────────────────────────────────────────────────--
	Rectangle {
		id: card
		width:  Math.min(920, root.width - 80)
		height: Math.min(660, root.height - 120)
		anchors.centerIn: parent
		radius: ThemeManager.panelRadius + 4
		color:  ThemeManager.surfaceContainer
		border.width: 1
		border.color: ThemeManager.outlineVariant
		opacity: root.active ? 1 : 0
		scale:   root.active ? 1 : 0.96
		layer.enabled: true
		layer.effect: Elevation { level: 4 }
		Behavior on opacity { NumberAnimation { duration: 150 } }
		Behavior on scale   { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }

		focus: root.active
		Keys.onEscapePressed: root._goBack()

		RowLayout {
			anchors.fill: parent
			spacing: 0

			// ── Sidebar ───────────────────────────────────────────────────────
			Rectangle {
				Layout.fillHeight: true
				Layout.preferredWidth: 188
				color: ThemeManager.surfaceContainerLow
				topLeftRadius: ThemeManager.panelRadius + 4
				bottomLeftRadius: ThemeManager.panelRadius + 4

				ColumnLayout {
					anchors.fill: parent
					anchors.margins: 12
					spacing: 4

					Text {
						text: I18n.tr("Settings")
						color: ThemeManager.onSurface
						font.family: ThemeManager.fontFamily
						font.pixelSize: ThemeManager.fontSizeLg
						font.bold: true
						Layout.bottomMargin: 8
						Layout.leftMargin: 6
					}

					Flickable {
						id: _sidebarScroll
						Layout.fillWidth: true
						Layout.fillHeight: true
						clip: true
						contentWidth: width
						contentHeight: _sidebarItems.implicitHeight
						boundsBehavior: Flickable.StopAtBounds
						ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

						ColumnLayout {
							id: _sidebarItems
							width: _sidebarScroll.width - (_sidebarScroll.contentHeight > _sidebarScroll.height ? 7 : 0)
							spacing: 4
							Repeater {
								model: root._cats
								delegate: Rectangle {
									required property var modelData
									Layout.fillWidth: true
									implicitHeight: 36
									radius: ThemeManager.chipRadius
									readonly property bool sel: root._groupFor(SettingsUi.category) === modelData.id
									color: sel ? ThemeManager.secondaryContainer
												: (_h.hovered ? ThemeManager.surfaceContainerHigh : "transparent")
									RowLayout {
										anchors.fill: parent
										anchors.leftMargin: 12; anchors.rightMargin: 12
										spacing: 10
										Text {
											Layout.preferredWidth: 22
											horizontalAlignment: Text.AlignHCenter
											text: modelData.icon
											color: sel ? ThemeManager.primary : ThemeManager.onSurfaceVariant
											font.family: ThemeManager.fontFamily
											font.pixelSize: 16
										}
										Text {
											Layout.fillWidth: true
											Layout.maximumWidth: parent.width - 38
											text: modelData.label
											color: sel ? ThemeManager.onSurface : ThemeManager.onSurfaceVariant
											font.family: ThemeManager.fontFamily
											font.pixelSize: ThemeManager.fontSizeMd
											maximumLineCount: 1
											elide: Text.ElideRight
											clip: true
										}
									}
									HoverHandler { id: _h }
									TapHandler { onTapped: SettingsUi.category = modelData.id }
								}
							}
						}
					}
				}
			}

			// ── Content pane ──────────────────────────────────────────────────
			Flickable {
				Layout.fillWidth: true
				Layout.fillHeight: true
				clip: true
				contentWidth: width
				contentHeight: _pane.implicitHeight
				boundsBehavior: Flickable.StopAtBounds

				ColumnLayout {
					id: _pane
					width: parent.width
					Component.onCompleted: {}

					// Breadcrumb navigation remains at the top of every page.
					RowLayout {
						Layout.fillWidth: true
						Layout.leftMargin: 20; Layout.rightMargin: 20; Layout.topMargin: 14; Layout.bottomMargin: 4
						spacing: 8
						Rectangle {
							implicitWidth: 32; implicitHeight: 32; radius: 16
							color: _backHover.hovered ? ThemeManager.surfaceContainerHigh : "transparent"
							Text { anchors.centerIn: parent; text: "󰁍"; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 18 }
							HoverHandler { id: _backHover }
							TapHandler { onTapped: root._goBack() }
						}
						Text {
							text: root._groupLabel(root._groupFor(SettingsUi.category))
							color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
							TapHandler { onTapped: SettingsUi.category = root._groupFor(SettingsUi.category) }
						}
						Text { visible: root._groupFor(SettingsUi.category) !== SettingsUi.category; text: "›"; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily }
						Text {
							visible: root._groupFor(SettingsUi.category) !== SettingsUi.category
							text: root._pageLabel(SettingsUi.category); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Item { Layout.fillWidth: true }
					}

					ColumnLayout {
						visible: root._groups[SettingsUi.category] !== undefined
						Layout.fillWidth: true; Layout.margins: 20; spacing: 10
						Text { text: root._groupLabel(SettingsUi.category); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeLg + 4; font.bold: true }
						Repeater {
							model: root._groups[SettingsUi.category] || []
							delegate: Rectangle {
								required property var modelData
								Layout.fillWidth: true; implicitHeight: 72; radius: ThemeManager.chipRadius + 3
								color: _pageHover.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow
								border.width: 1; border.color: ThemeManager.outlineVariant
								RowLayout {
									anchors { fill: parent; margins: 14 }
									spacing: 14
									Text {
										Layout.preferredWidth: 34
										horizontalAlignment: Text.AlignHCenter
										text: modelData.icon
										color: ThemeManager.primary
										font.family: ThemeManager.fontFamily
										font.pixelSize: 23
									}
									ColumnLayout {
										Layout.fillWidth: true; spacing: 2
										Text {
											Layout.fillWidth: true
											horizontalAlignment: Text.AlignLeft
											text: modelData.label
											color: ThemeManager.onSurface
											font.family: ThemeManager.fontFamily
											font.pixelSize: ThemeManager.fontSizeMd
											font.bold: true
											maximumLineCount: 1
											elide: Text.ElideRight
										}
										Text {
											Layout.fillWidth: true
											horizontalAlignment: Text.AlignLeft
											text: modelData.sub
											color: ThemeManager.onSurfaceVariant
											font.family: ThemeManager.fontFamily
											font.pixelSize: 10
											maximumLineCount: 1
											elide: Text.ElideRight
										}
									}
									Text {
										Layout.preferredWidth: 20
										horizontalAlignment: Text.AlignHCenter
										text: "󰅂"
										color: ThemeManager.onSurfaceVariant
										font.family: ThemeManager.fontFamily
										font.pixelSize: 18
									}
								}
								HoverHandler { id: _pageHover }
								TapHandler { onTapped: SettingsUi.category = modelData.id }
							}
						}
					}

					// User ----------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "user"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 12

						SettingSection { text: I18n.tr("User profile") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Choose the image shown in the dashboard and on the lock screen.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						Text {
							Layout.fillWidth: true
							Layout.topMargin: 6
							text: I18n.tr("Default avatars")
							color: ThemeManager.onSurface
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeMd
							font.weight: Font.Medium
						}

						Flow {
							Layout.fillWidth: true
							spacing: 12
							Repeater {
								model: ProfileService.presets
								delegate: Column {
									required property var modelData
									spacing: 5
									width: 104

									ClippingRectangle {
										id: _presetAvatar
										anchors.horizontalCenter: parent.horizontalCenter
										width: 88; height: 88; radius: 44
										color: ThemeManager.surfaceContainerHigh
										border.width: ProfileService.selectedPreset === modelData.id ? 3 : 1
										border.color: ProfileService.selectedPreset === modelData.id
											? ThemeManager.primary : ThemeManager.outlineVariant
										Image {
											anchors.fill: parent
											source: modelData.source
											fillMode: Image.PreserveAspectCrop
											asynchronous: true
										}
										Rectangle {
											anchors.fill: parent
											radius: _presetAvatar.radius
											color: _presetHover.hovered ? Qt.rgba(1, 1, 1, 0.10) : "transparent"
										}
										HoverHandler { id: _presetHover; cursorShape: Qt.PointingHandCursor }
										TapHandler { onTapped: ProfileService.selectPreset(modelData.id, modelData.source) }
									}

									Text {
										width: parent.width
										horizontalAlignment: Text.AlignHCenter
										text: modelData.name
										color: ProfileService.selectedPreset === modelData.id
											? ThemeManager.primary : ThemeManager.onSurfaceVariant
										font.family: ThemeManager.fontFamily
										font.pixelSize: ThemeManager.fontSizeSm
										elide: Text.ElideRight
									}
								}
							}
						}

						SettingSection { text: I18n.tr("Personal image"); Layout.topMargin: 8 }

						ClippingRectangle {
							Layout.alignment: Qt.AlignHCenter
							Layout.topMargin: 8
							width: 144; height: 144; radius: 72
							color: ThemeManager.surfaceContainerHigh
							border.width: 2
							border.color: ThemeManager.primary
							Image {
								id: _profilePreview
								anchors.fill: parent
								source: ProfileService.avatarUrl
								fillMode: Image.PreserveAspectCrop
								asynchronous: true
								cache: false
								visible: status === Image.Ready
							}
							Text {
								anchors.centerIn: parent
								visible: _profilePreview.status !== Image.Ready
								text: "󰀄"
								color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily
								font.pixelSize: 64
							}
						}

						Row {
							Layout.alignment: Qt.AlignHCenter
							spacing: 10
							SettingBtn { label: I18n.tr("Choose image"); onClicked: _avatarDialog.open() }
							SettingBtn {
								visible: ProfileService.available
								label: I18n.tr("Remove image")
								danger: true
								onClicked: ProfileService.removeAvatar()
							}
						}

						Text {
							visible: ProfileService.available
							Layout.alignment: Qt.AlignHCenter
							text: I18n.tr("Profile image updated")
							color: ThemeManager.primary
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}
					}

					// Security ----------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "security"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 10

						SettingSection { text: I18n.tr("Automatic screen lock") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Protect your session by showing the Nodalix lock screen after a period without activity.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						SettingToggle {
							label: I18n.tr("Lock automatically")
							sub: I18n.tr("Require your password when you return")
							path: "security.autoLock.enabled"
							def: true
						}

						SettingToggle {
							label: I18n.tr("Pause in fullscreen applications")
							sub: I18n.tr("Prevents locking while playing with a controller or watching video")
							path: "security.autoLock.pauseFullscreen"
							def: true
						}

						Text {
							visible: AutoLockService.fullscreenProtectionActive
							Layout.fillWidth: true
							text: I18n.tr("Fullscreen protection active · automatic lock paused")
							color: ThemeManager.primary
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						SettingSeg {
							label: I18n.tr("Lock after")
							sub: I18n.tr("Time without user activity")
							enabled: SettingsService.get("security.autoLock.enabled", true)
							options: [
								I18n.tr("1 min"), I18n.tr("5 min"), I18n.tr("10 min"),
								I18n.tr("15 min"), I18n.tr("30 min"), I18n.tr("1 hour")
							]
							keys: ["1", "5", "10", "15", "30", "60"]
							path: "security.autoLock.minutes"
							def: "10"
						}

						Rectangle {
							Layout.fillWidth: true
							Layout.topMargin: 8
							implicitHeight: _idleInfo.implicitHeight + 24
							radius: ThemeManager.chipRadius + 2
							color: ThemeManager.surfaceContainerLow
							border.width: 1
							border.color: ThemeManager.outlineVariant

							RowLayout {
								anchors { fill: parent; margins: 12 }
								spacing: 10
								Text {
									text: "󰈈"
									color: ThemeManager.primary
									font.family: ThemeManager.fontFamily
									font.pixelSize: 18
								}
								Text {
									id: _idleInfo
									Layout.fillWidth: true
									text: I18n.tr("Videos, calls and applications that keep the screen awake are respected. Fullscreen games remain protected even when the controller is not reported as activity.")
									wrapMode: Text.WordWrap
									color: ThemeManager.onSurfaceVariant
									font.family: ThemeManager.fontFamily
									font.pixelSize: 10
								}
							}
						}
					}

					// Appearance ----------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "appearance"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Appearance") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Choose the language used by the shell, system and spell checker.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}
						SettingSeg {
							label: I18n.tr("Language")
							enabled: !SystemLocaleService.busy
							options: [I18n.tr("English"), I18n.tr("Spanish")]
							keys: ["en", "es"]
							path: "general.language"
							def: "en"
							applyFn: language => SystemLocaleService.apply(language)
						}
						Text {
							visible: SystemLocaleService.statusText !== ""
							Layout.fillWidth: true
							text: SystemLocaleService.statusText
							wrapMode: Text.WordWrap
							color: SystemLocaleService.failed ? ThemeManager.error : ThemeManager.primary
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						SettingSeg {
							label: I18n.tr("System font")
							sub: I18n.tr("Nodalix and compatible applications")
							enabled: !FontService.busy
							options: [I18n.tr("Nodalix Mono"), I18n.tr("Nodalix Proportional"), "Noto Sans", "Adwaita"]
							keys: ["JetBrainsMono Nerd Font", "JetBrainsMono Nerd Font Propo", "Noto Sans", "Adwaita Sans"]
							path: "appearance.fontFamily"
							def: "JetBrainsMono Nerd Font"
							applyFn: fontFamily => FontService.apply(fontFamily)
						}
						Text {
							visible: FontService.statusText !== ""
							Layout.fillWidth: true
							text: FontService.statusText
							wrapMode: Text.WordWrap
							color: FontService.failed ? ThemeManager.error : ThemeManager.primary
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						SettingSeg {
							label: I18n.tr("Overall style")
							sub: I18n.tr("Frame border · top bar only · floating bar")
							options: [I18n.tr("Frame"), I18n.tr("Top bar"), I18n.tr("Islands")]
							keys: ["frame", "topbar", "islands"]
							path: "appearance.mode"; def: "frame"
						}
						SettingSlider { label: I18n.tr("Bar height"); path: "bar.height"; def: 40; from: 28; to: 56; unit: "px" }
						SettingSlider { label: I18n.tr("Panel radius"); path: "appearance.panelRadius"; def: 16; from: 0; to: 28; unit: "px" }
						SettingSlider { label: I18n.tr("Base font size"); path: "appearance.fontSize"; def: 13; from: 10; to: 18; unit: "pt" }
						SettingToggle { label: I18n.tr("Panel blur"); sub: I18n.tr("Background blur behind panels"); path: "appearance.blur"; def: true }

						SettingSection { text: I18n.tr("Theme") }
						Flow {
							Layout.fillWidth: true
							spacing: 8
							Repeater {
								model: ThemeManager.pickerThemes
								delegate: Rectangle {
									required property var modelData
									readonly property bool sel: ThemeManager.activeId === modelData.id
									implicitWidth: _tn.implicitWidth + 28
									implicitHeight: 34
									radius: ThemeManager.chipRadius
									color: sel ? ThemeManager.secondaryContainer
												: (_th.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow)
									border.width: sel ? 2 : 1
									border.color: sel ? ThemeManager.primary : ThemeManager.outlineVariant
									Row {
										anchors.centerIn: parent
										spacing: 7
										Rectangle { width: 12; height: 12; radius: 6; anchors.verticalCenter: parent.verticalCenter
													color: modelData.dark ? "#222" : "#eee"; border.width: 1; border.color: ThemeManager.outlineVariant }
										Text { id: _tn; text: modelData.name
												color: sel ? ThemeManager.onSurface : ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
									}
									HoverHandler { id: _th }
									TapHandler { onTapped: ThemeManager.setTheme(modelData.id) }
								}
							}
						}
						// Theme actions
						Row {
							Layout.topMargin: 4
							spacing: 8
							SettingBtn { label: I18n.tr("New");       onClicked: { root._themeAction = "new";       _nameInput.text = ""; _nameInput.forceActiveFocus() } }
							SettingBtn { label: I18n.tr("Duplicate"); onClicked: { root._themeAction = "duplicate"; _nameInput.text = ThemeManager.name + " copy"; _nameInput.forceActiveFocus() } }
							SettingBtn { enabled: ThemeManager._isUser(ThemeManager.activeId); label: I18n.tr("Rename"); onClicked: { root._themeAction = "rename"; _nameInput.text = ThemeManager.name; _nameInput.forceActiveFocus() } }
							SettingBtn { enabled: ThemeManager._isUser(ThemeManager.activeId); label: I18n.tr("Delete"); danger: true; onClicked: ThemeManager.deleteTheme(ThemeManager.activeId) }
						}
						// Name entry — only while a New/Duplicate/Rename is pending
						RowLayout {
							visible: root._themeAction !== ""
							Layout.fillWidth: true; Layout.topMargin: 4
							spacing: 8
							TextField {
								id: _nameInput
								Layout.fillWidth: true; implicitHeight: 30
								placeholderText: I18n.tr("Theme name")
								placeholderTextColor: ThemeManager.onSurfaceVariant
								color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
								leftPadding: 10; rightPadding: 10
								background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
														border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
								onAccepted: root._confirmTheme(text)
								Keys.onEscapePressed: root._themeAction = ""
							}
							SettingBtn {
								label: root._themeAction === "rename" ? "Rename"
									: root._themeAction === "duplicate" ? "Duplicate" : "Create"
								onClicked: root._confirmTheme(_nameInput.text)
							}
							SettingBtn { label: I18n.tr("Cancel"); onClicked: root._themeAction = "" }
						}

						// ── Theme designer (colors) ────────────────────────────
						SettingSection { text: I18n.tr("Designer") }
						Text {
							visible: !ThemeManager._isUser(ThemeManager.activeId)
							Layout.fillWidth: true
							text: I18n.tr("Built-in themes are read-only — Duplicate one to edit its colors.")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
						ColumnLayout {
							visible: ThemeManager._isUser(ThemeManager.activeId)
							Layout.fillWidth: true; Layout.topMargin: 4
							spacing: 4
							Repeater {
								model: ThemeManager.editableRoles
								delegate: SettingColor { required property var modelData; role: modelData }
							}
						}
						SettingText {
							Layout.topMargin: 6
							label: I18n.tr("Import (.json)"); sub: I18n.tr("Path → new theme")
							path: "_importPath"; def: ""; placeholder: I18n.tr("/path/theme.json")
						}
						Row {
							Layout.topMargin: 6
							spacing: 10
							SettingBtn { label: I18n.tr("Import"); onClicked: ThemeManager.importTheme(SettingsService.get("_importPath", ""), "imported") }
							SettingBtn { label: I18n.tr("Export current theme"); onClicked: _exportTheme.running = true }
						}
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Exports the active theme's colors to ~/.local/state/nodalix/exports/")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
						Process {
							id: _exportTheme
							command: ["sh", "-c",
								"d=\"" + (Paths.stateDir + "/exports") + "\"; " +
								"mkdir -p \"$d\" && printf '%s' '" +
								JSON.stringify(ThemeManager.themeData).replace(/'/g, "'\\''") +
								"' > \"$d/" + ThemeManager.activeId + ".json\""]
						}
					}

					// Bar -----------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "bar"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Clock") }
						SettingToggle { label: I18n.tr("24-hour clock"); path: "bar.clock.use24h"; def: true }
						SettingToggle { label: I18n.tr("Show seconds"); path: "bar.clock.seconds"; def: false }
						SettingSection { text: I18n.tr("Widgets") }
						SettingToggle { label: I18n.tr("Launcher button"); path: "bar.widgets.launcher"; def: true }
						SettingToggle { label: I18n.tr("Active window title"); path: "bar.widgets.windowTitle"; def: true }
						SettingToggle { label: I18n.tr("Media mini indicator"); path: "bar.widgets.media"; def: false }
						SettingToggle { label: I18n.tr("Status row (battery/wifi/bt/vol)"); path: "bar.widgets.status"; def: true }
						SettingSection { text: I18n.tr("Workspaces") }
						SettingToggle { label: I18n.tr("Show numbers"); sub: I18n.tr("Off = dots"); path: "bar.workspaces.numbers"; def: false }
						SettingToggle { label: I18n.tr("Hide special workspaces"); path: "bar.workspaces.hideSpecial"; def: true }
						SettingToggle { label: I18n.tr("Unified workspaces"); sub: I18n.tr("Shared across screens (off = per-monitor)"); path: "workspaces.unified"; def: false }
					}

					// Media ---------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "media"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Media player") }
						SettingToggle { label: I18n.tr("Audio visualizer"); dep: "cava"; path: "media.visualizer"; def: true }
						SettingToggle { label: I18n.tr("Bongo cat"); path: "media.bongo"; def: true }
						SettingToggle { label: I18n.tr("YouTube Music companion"); sub: I18n.tr("Realtime integration (ytmdesktop companion server)"); path: "media.ytm"; def: true }
					}

					// Notifications -------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "notifications"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Notifications") }
						SettingToggle { label: I18n.tr("Click notification opens app"); path: "notifications.clickOpensApp"; def: true }
						SettingToggle { label: I18n.tr("Start in Do Not Disturb"); path: "notifications.dndDefault"; def: false }
						SettingSlider { label: I18n.tr("Toast timeout"); path: "notifications.toastMs"; def: 5000; from: 2000; to: 15000; unit: "ms" }
						SettingSlider { label: I18n.tr("Max toast stack"); path: "notifications.toastMax"; def: 5; from: 1; to: 10; unit: "" }
					}

					// Weather -------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "weather"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Weather") }
						SettingText { label: I18n.tr("Location"); sub: I18n.tr("City or lat,lon — empty = auto by IP"); path: "weather.location"; def: "Dijon"; placeholder: I18n.tr("auto") }
						SettingToggle { label: I18n.tr("Fahrenheit"); sub: I18n.tr("Off = Celsius"); path: "weather.fahrenheit"; def: false }
						SettingSlider { label: I18n.tr("Refresh interval"); path: "weather.refreshMin"; def: 30; from: 5; to: 120; unit: "min" }
					}

					// Keybindings ---------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "keybindings"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Keybindings") }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 4
							text: I18n.tr("Shortcuts for the shell's actions. Unbound by default. Type a Hyprland combo, e.g. SUPER + R or SUPER + SHIFT + L. Saving reloads Hyprland to apply. Requires hypr/quickshell.lua (Lua config).")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: BindingService.actions
							delegate: RowLayout {
								required property var modelData
								Layout.fillWidth: true
								spacing: 10
								Text {
									Layout.fillWidth: true
									text: modelData.label
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									elide: Text.ElideRight
								}
								TextField {
									id: _bindField
									Layout.preferredWidth: 180; implicitHeight: 28
									text: SettingsService.get("binds." + modelData.key, "")
									placeholderText: I18n.tr("Unbound")
									placeholderTextColor: ThemeManager.onSurfaceVariant
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									leftPadding: 8; rightPadding: 8
									background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
															border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
									onEditingFinished: if (text !== SettingsService.get("binds." + modelData.key, "")) BindingService.setCombo(modelData.key, text)
								}
								SettingBtn {
									label: I18n.tr("Clear"); danger: true
									enabled: SettingsService.get("binds." + modelData.key, "") !== ""
									onClicked: { _bindField.text = ""; BindingService.setCombo(modelData.key, "") }
								}
							}
						}
					}

					// Tray ----------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "tray"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6

						// Custom entries (non-SNI apps) -----------------------------
						SettingSection { text: I18n.tr("Custom entries") }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 2
							text: I18n.tr("Pin any app to the tray, even ones without tray support. Each entry is one clickable icon.")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}

						Repeater {
							model: root._trayCustomList
							delegate: Rectangle {
								id: _ce
								required property var modelData
								required property int index
								readonly property bool _ws: (modelData.action ?? "run") === "ws"
								readonly property bool editing: root._trayEditIdx === index

								function _iconSrc() {
									const ic = String(_ce.modelData.icon ?? "")
									if (ic === "") return ""
									return (ic.startsWith("/") || ic.indexOf("://") >= 0) ? ic : Quickshell.iconPath(ic, true)
								}

								Layout.fillWidth: true
								Layout.topMargin: 6
								radius: ThemeManager.panelRadius
								color: ThemeManager.surfaceContainerHigh
								border.width: 1; border.color: editing ? ThemeManager.primary : ThemeManager.outlineVariant
								implicitHeight: (editing ? _ceCol.implicitHeight : _ceRow.implicitHeight) + 24

								// ── Collapsed summary ──────────────────────────────
								RowLayout {
									id: _ceRow
									visible: !_ce.editing
									anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 12; rightMargin: 12 }
									spacing: 10
									Rectangle {
										implicitWidth: 30; implicitHeight: 30; radius: 6
										color: ThemeManager.surfaceContainer
										border.width: 1; border.color: ThemeManager.outlineVariant
										IconImage { anchors.centerIn: parent; implicitSize: 20; source: _ce._iconSrc() }
									}
									ColumnLayout {
										Layout.fillWidth: true; spacing: 1
										Text {
											Layout.fillWidth: true; elide: Text.ElideRight
											text: (_ce.modelData.name && _ce.modelData.name !== "") ? _ce.modelData.name : "Unnamed"
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; font.bold: true
										}
										Text {
											Layout.fillWidth: true; elide: Text.ElideRight
											text: (_ce._ws ? "Toggle workspace · " : "Run · ") + ((_ce.modelData.value && _ce.modelData.value !== "") ? _ce.modelData.value : "not set")
											color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11
										}
									}
									SettingBtn { label: I18n.tr("Edit"); onClicked: root._trayEditIdx = _ce.index }
									SettingBtn { label: I18n.tr("Remove"); danger: true; onClicked: root._trayCustomRemove(_ce.index) }
								}

								// ── Expanded editor ────────────────────────────────
								ColumnLayout {
									id: _ceCol
									visible: _ce.editing
									anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
									spacing: 10

									// Header: live icon preview + name field
									RowLayout {
										Layout.fillWidth: true
										spacing: 10
										Rectangle {
											implicitWidth: 30; implicitHeight: 30; radius: 6
											color: ThemeManager.surfaceContainer
											border.width: 1; border.color: ThemeManager.outlineVariant
											IconImage { anchors.centerIn: parent; implicitSize: 20; source: _ce._iconSrc() }
										}
										ColumnLayout {
											Layout.fillWidth: true; spacing: 2
											Text { text: I18n.tr("Label (tooltip on hover)"); color: ThemeManager.onSurfaceVariant
													font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
											TextField {
												Layout.fillWidth: true; implicitHeight: 28
												text: _ce.modelData.name ?? ""; placeholderText: I18n.tr("e.g. Firefox")
												placeholderTextColor: ThemeManager.onSurfaceVariant
												color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
												leftPadding: 8; rightPadding: 8
												background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer
																		border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
												onEditingFinished: if (text !== (_ce.modelData.name ?? "")) root._trayCustomSet(_ce.index, "name", text)
											}
										}
									}

									// Icon field
									ColumnLayout {
										Layout.fillWidth: true; spacing: 2
										Text { text: I18n.tr("Icon — freedesktop name (e.g. firefox, spotify) or /path/to/icon.png")
												color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
										TextField {
											Layout.fillWidth: true; implicitHeight: 28
											text: _ce.modelData.icon ?? ""; placeholderText: I18n.tr("firefox")
											placeholderTextColor: ThemeManager.onSurfaceVariant
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
											leftPadding: 8; rightPadding: 8
											background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer
																	border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
											onEditingFinished: if (text !== (_ce.modelData.icon ?? "")) root._trayCustomSet(_ce.index, "icon", text)
										}
									}

									// Action chooser (segmented) + value field
									ColumnLayout {
										Layout.fillWidth: true; spacing: 4
										Text { text: I18n.tr("On left-click"); color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
										Row {
											spacing: 0
											Repeater {
												model: [{ k: "run", t: I18n.tr("Run a command") }, { k: "ws", t: I18n.tr("Toggle workspace") }]
												delegate: Rectangle {
													required property var modelData
													required property int index
													readonly property bool sel: (_ce.modelData.action ?? "run") === modelData.k
													implicitWidth: _segT.implicitWidth + 24; implicitHeight: 28
													topLeftRadius:    index === 0 ? ThemeManager.chipRadius : 0
													bottomLeftRadius: index === 0 ? ThemeManager.chipRadius : 0
													topRightRadius:    index === 1 ? ThemeManager.chipRadius : 0
													bottomRightRadius: index === 1 ? ThemeManager.chipRadius : 0
													color: sel ? ThemeManager.primary : ThemeManager.surfaceContainer
													border.width: 1; border.color: sel ? ThemeManager.primary : ThemeManager.outlineVariant
													Text { id: _segT; anchors.centerIn: parent; text: modelData.t
															color: parent.sel ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
															font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
													TapHandler { onTapped: root._trayCustomSet(_ce.index, "action", modelData.k) }
												}
											}
										}
										TextField {
											Layout.fillWidth: true; Layout.topMargin: 2; implicitHeight: 28
											text: _ce.modelData.value ?? ""
											placeholderText: _ce._ws ? "special workspace name (e.g. spotify)" : "command to run (e.g. firefox)"
											placeholderTextColor: ThemeManager.onSurfaceVariant
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
											leftPadding: 8; rightPadding: 8
											background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer
																	border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
											onEditingFinished: if (text !== (_ce.modelData.value ?? "")) root._trayCustomSet(_ce.index, "value", text)
										}
										Text {
											Layout.fillWidth: true; wrapMode: Text.WordWrap
											text: _ce._ws
												? "Click peeks/hides that Hyprland special workspace (park the app there via a window rule)."
												: "Click runs this shell command (launches or focuses the app)."
											color: ThemeManager.onSurfaceVariant
											font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11
										}
									}

									// Footer: Remove + Validate (collapse)
									RowLayout {
										Layout.fillWidth: true; Layout.topMargin: 2
										SettingBtn { label: I18n.tr("Remove"); danger: true; onClicked: root._trayCustomRemove(_ce.index) }
										Item { Layout.fillWidth: true }
										SettingBtn { label: I18n.tr("Validate"); onClicked: root._trayEditIdx = -1 }
									}
								}
							}
						}

						SettingBtn { Layout.topMargin: 8; label: I18n.tr("+  Add entry"); onClicked: root._trayCustomAdd() }

						SettingSection { text: I18n.tr("System tray"); Layout.topMargin: 12 }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 4
							text: I18n.tr("Per app: hide it, or set a special workspace (left-click then toggles that workspace instead of activating).")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: SystemTray.items
							delegate: RowLayout {
								required property var modelData
								Layout.fillWidth: true
								spacing: 10
								IconImage { implicitSize: 18; source: modelData.icon ?? "" }
								Text {
									Layout.fillWidth: true
									text: (modelData.tooltipTitle && modelData.tooltipTitle !== "") ? modelData.tooltipTitle
										: ((modelData.title && modelData.title !== "") ? modelData.title : (modelData.id ?? "item"))
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									elide: Text.ElideRight
								}
								TextField {
									Layout.preferredWidth: 104; implicitHeight: 26
									text: root._trayWs(modelData); placeholderText: I18n.tr("workspace")
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									leftPadding: 8; rightPadding: 8
									background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
															border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
									onEditingFinished: if (text !== root._trayWs(modelData)) root._traySetWs(modelData, text)
								}
								SettingBtn {
									readonly property bool _hidden: root._trayIsHidden(modelData)
									label: _hidden ? "Hidden" : "Visible"; danger: _hidden
									onClicked: root._trayToggleHide(modelData)
								}
							}
						}
						Text {
							visible: (SystemTray.items?.values ?? []).length === 0
							text: I18n.tr("No tray items.")
							color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
					}

					// Tools ---------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "tools"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Tools toolbar") }
						SettingToggle { label: I18n.tr("Enable toolbar"); path: "tools.enabled"; def: true }
						SettingToggle { label: I18n.tr("Wallpaper picker"); sub: I18n.tr("Built-in background/theme tool"); dep: "matugen"; path: "tools.wallpaper"; def: true }

						SettingSection { text: I18n.tr("Custom tools"); Layout.topMargin: 12 }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 2
							text: I18n.tr("Add your own rail buttons — each runs a command. Examples: a file manager or a screen recorder.")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: root._toolCustomList
							delegate: Rectangle {
								id: _tc
								required property var modelData
								required property int index
								readonly property bool editing: root._toolEditIdx === index

								Layout.fillWidth: true
								Layout.topMargin: 6
								radius: ThemeManager.panelRadius
								color: ThemeManager.surfaceContainerHigh
								border.width: 1; border.color: editing ? ThemeManager.primary : ThemeManager.outlineVariant
								implicitHeight: (editing ? _tcCol.implicitHeight : _tcRow.implicitHeight) + 24

								// Collapsed summary
								RowLayout {
									id: _tcRow
									visible: !_tc.editing
									anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 12; rightMargin: 12 }
									spacing: 10
									Text {
										text: _tc.modelData.icon || "󰘔"; color: ThemeManager.onSurface
										font.family: ThemeManager.fontFamily; font.pixelSize: 22
										Layout.preferredWidth: 28; horizontalAlignment: Text.AlignHCenter
									}
									ColumnLayout {
										Layout.fillWidth: true; spacing: 1
										Text {
											Layout.fillWidth: true; elide: Text.ElideRight
											text: (_tc.modelData.name && _tc.modelData.name !== "") ? _tc.modelData.name : "Unnamed"
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; font.bold: true
										}
										Text {
											Layout.fillWidth: true; elide: Text.ElideRight
											text: (_tc.modelData.command && _tc.modelData.command !== "") ? _tc.modelData.command : "no command"
											color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11
										}
									}
									SettingBtn { label: I18n.tr("Edit"); onClicked: root._toolEditIdx = _tc.index }
									SettingBtn { label: I18n.tr("Remove"); danger: true; onClicked: root._toolCustomRemove(_tc.index) }
								}

								// Expanded editor
								ColumnLayout {
									id: _tcCol
									visible: _tc.editing
									anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
									spacing: 10

									ColumnLayout {
										Layout.fillWidth: true; spacing: 2
										Text { text: I18n.tr("Name (tooltip)"); color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
										TextField {
											Layout.fillWidth: true; implicitHeight: 28
											text: _tc.modelData.name ?? ""; placeholderText: I18n.tr("e.g. Files")
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
											leftPadding: 8; rightPadding: 8
											background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer
																	border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
											onEditingFinished: if (text !== (_tc.modelData.name ?? "")) root._toolCustomSet(_tc.index, "name", text)
										}
									}
									ColumnLayout {
										Layout.fillWidth: true; spacing: 2
										Text { text: I18n.tr("Command"); color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
										TextField {
											Layout.fillWidth: true; implicitHeight: 28
											text: _tc.modelData.command ?? ""; placeholderText: I18n.tr("kitty -e yazi")
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
											leftPadding: 8; rightPadding: 8
											background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer
																	border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
											onEditingFinished: if (text !== (_tc.modelData.command ?? "")) root._toolCustomSet(_tc.index, "command", text)
										}
									}
									// Icon picker — grid of glyphs
									ColumnLayout {
										Layout.fillWidth: true; spacing: 4
										Text { text: I18n.tr("Icon"); color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeXs ?? 11 }
										Flow {
											Layout.fillWidth: true; spacing: 4
											Repeater {
												model: root._toolIcons
												delegate: Rectangle {
													required property var modelData
													readonly property bool sel: (_tc.modelData.icon || "") === modelData
													width: 34; height: 34; radius: 8
													color: sel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.22)
																: ThemeManager.surfaceContainer
													border.width: 1; border.color: sel ? ThemeManager.primary : ThemeManager.outlineVariant
													Text { anchors.centerIn: parent; text: modelData
															color: parent.sel ? ThemeManager.primary : ThemeManager.onSurfaceVariant
															font.family: ThemeManager.fontFamily; font.pixelSize: 20 }
													TapHandler { onTapped: root._toolCustomSet(_tc.index, "icon", modelData) }
												}
											}
										}
									}

									RowLayout {
										Layout.fillWidth: true; Layout.topMargin: 2
										SettingBtn { label: I18n.tr("Remove"); danger: true; onClicked: root._toolCustomRemove(_tc.index) }
										Item { Layout.fillWidth: true }
										SettingBtn { label: I18n.tr("Validate"); onClicked: root._toolEditIdx = -1 }
									}
								}
							}
						}
						SettingBtn { Layout.topMargin: 8; label: I18n.tr("+  Add tool"); onClicked: root._toolCustomAdd() }
					}

					// Wallpaper -----------------------------------------------------
					ColumnLayout {
						id: _wpPane
						visible: SettingsUi.category === "wallpaper"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6

						readonly property var _localList: WallpaperService.wallpapers
						readonly property var _favList:   WallpaperService.favorites
						readonly property var _shownList: root._wpTab === "favorites" ? _favList : (root._wpTab === "animated" ? WallpaperService.animatedWallpapers : _localList)
						property int _rotAnchor: -1   // last-clicked index, for shift-range rotation select

						SettingToggle {
							label: I18n.tr("Pause animated wallpapers in fullscreen")
							sub: I18n.tr("Pauses only the monitor with fullscreen content")
							path: "wallpaper.pauseAnimatedFullscreen"
							def: true
						}

						Text {
							visible: !WallpaperService.available
							Layout.fillWidth: true
							text: I18n.tr("hyprpaper not installed — the wallpaper switcher is disabled.")
							wrapMode: Text.WordWrap; color: ThemeManager.error
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}

						Text {
							visible: root._wpTab === "animated" && !WallpaperService.animatedAvailable
							Layout.fillWidth: true
							text: I18n.tr("Install mpvpaper to use animated wallpapers.")
							wrapMode: Text.WordWrap; color: ThemeManager.error
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}

						// ── Tabs ──────────────────────────────────────────────────
						RowLayout {
							Layout.fillWidth: true
							spacing: 6
							Repeater {
								model: [ { id: "local", label: I18n.tr("Local") }, { id: "animated", label: I18n.tr("Animated") }, { id: "favorites", label: I18n.tr("Favorites") }, { id: "browse", label: I18n.tr("Browse") } ]
								delegate: Rectangle {
									required property var modelData
									readonly property bool sel: root._wpTab === modelData.id
									implicitWidth: _wtl.implicitWidth + 24; implicitHeight: 30
									radius: ThemeManager.chipRadius
									color: sel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.18)
												: (_wtMa.containsMouse ? Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.08) : "transparent")
									Text {
										id: _wtl; anchors.centerIn: parent; text: modelData.label
										color: parent.sel ? ThemeManager.primary : ThemeManager.onSurface
										font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									}
									MouseArea { id: _wtMa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root._wpTab = modelData.id }
								}
							}
							Item { Layout.fillWidth: true }
							SettingBtn { label: I18n.tr("Refresh"); onClicked: WallpaperService.refresh() }
						}

						Text {
							visible: root._wpTab !== "browse" && _wpPane._shownList.length === 0
							Layout.topMargin: 8
							text: root._wpTab === "favorites" ? I18n.tr("No favorites yet — tap the heart on a wallpaper.")
									: (root._wpTab === "animated" ? I18n.tr("No animated wallpapers yet.") : I18n.tr("No wallpapers in your Nodalix Pictures folder."))
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
							opacity: 0.7
						}

						// ── Local / Favorites thumbnail grid ──────────────────────
						Flow {
							visible: root._wpTab !== "browse"
							Layout.fillWidth: true
							Layout.topMargin: 6
							spacing: 8
							Repeater {
								model: root._wpTab === "browse" ? [] : _wpPane._shownList
								delegate: ClippingRectangle {
									id: _tile
									required property var modelData
									required property int index
									readonly property string _path: "" + modelData
									readonly property bool _isVideo: root._wpTab === "animated"
									readonly property bool _isCurrent: WallpaperService.current === _path && WallpaperService.currentAnimated === _isVideo
									readonly property bool _inRot: WallpaperService.isInRotation(_path)
									width: 168; height: 96
									radius: ThemeManager.chipRadius
									color: ThemeManager.surfaceContainerHigh

									Image {
										visible: !_tile._isVideo
										anchors.fill: parent
										source: !_tile._isVideo ? ("file://" + _tile._path) : ""
										fillMode: Image.PreserveAspectCrop
										asynchronous: true; cache: false
										sourceSize.width: 336
									}
									Image {
										visible: _tile._isVideo
										anchors.fill: parent
										source: _tile._isVideo ? ("file://" + WallpaperService.thumbnailFor(_tile._path)) : ""
										fillMode: Image.PreserveAspectCrop
										asynchronous: true; cache: false
										sourceSize.width: 336
									}
									Text {
										visible: _tile._isVideo
										anchors.centerIn: parent
										text: "󰕧"
										color: "white"
										style: Text.Outline
										styleColor: "black"
										font.family: ThemeManager.fontFamily
										font.pixelSize: 30
									}
									// Border: current wallpaper (primary) or in-rotation (tertiary)
									Rectangle {
										anchors.fill: parent; radius: _tile.radius; color: "transparent"
										border.width: (_tile._isCurrent || _tile._inRot) ? 2 : 0
										border.color: _tile._isCurrent ? ThemeManager.primary : ThemeManager.tertiary
									}
									// Hover darken
									Rectangle {
										anchors.fill: parent; radius: _tile.radius
										color: Qt.rgba(0, 0, 0, _tileMa.containsMouse ? 0.18 : 0)
									}
									// Rotation badge (bottom-left) — membership set via ctrl/shift-click
									Rectangle {
										visible: _tile._inRot
										anchors { left: parent.left; bottom: parent.bottom; margins: 5 }
										width: 20; height: 20; radius: 10
										color: Qt.rgba(ThemeManager.tertiary.r, ThemeManager.tertiary.g, ThemeManager.tertiary.b, 0.9)
										Text { anchors.centerIn: parent; text: "󰑖"; color: ThemeManager.onTertiary
												font.family: ThemeManager.fontFamily; font.pixelSize: 12 }
									}
									// Click: plain = set; ctrl = toggle rotation; shift = range-add to rotation
									MouseArea {
										id: _tileMa
										anchors.fill: parent
										hoverEnabled: true; cursorShape: Qt.PointingHandCursor
										onClicked: (m) => {
											if (_tile._isVideo) {
												WallpaperService.commitAnimated(_tile._path)
											} else if (m.modifiers & Qt.ControlModifier) {
												WallpaperService.toggleRotation(_tile._path)
												_wpPane._rotAnchor = _tile.index
											} else if (m.modifiers & Qt.ShiftModifier) {
												const list = _wpPane._shownList
												const a = _wpPane._rotAnchor >= 0 ? _wpPane._rotAnchor : _tile.index
												const lo = Math.min(a, _tile.index), hi = Math.max(a, _tile.index)
												const r = (WallpaperService.rotationPaths || []).slice()
												for (let i = lo; i <= hi; i++) {
													const pp = "" + list[i]
													if (r.indexOf(pp) < 0) r.push(pp)
												}
												WallpaperService.setRotation(r)
											} else {
												WallpaperService.commit(_tile._path)
												_wpPane._rotAnchor = _tile.index
											}
										}
									}
									// Favorite button (on top of the tile MouseArea so it stays clickable)
									WpTileBtn {
										visible: !_tile._isVideo
										anchors { top: parent.top; right: parent.right; margins: 5 }
										icon: WallpaperService.isFavorite(_tile._path) ? "󰋑" : "󰋕"
										active: WallpaperService.isFavorite(_tile._path)
										onClicked: WallpaperService.toggleFavorite(_tile._path)
									}
								}
							}
						}

						// ── Browse (Wallhaven) ────────────────────────────────────
						ColumnLayout {
							visible: root._wpTab === "browse"
							Layout.fillWidth: true
							Layout.topMargin: 6
							spacing: 6

							RowLayout {
								Layout.fillWidth: true
								spacing: 6
								TextField {
									id: _whSearch
									Layout.fillWidth: true; implicitHeight: 30
									placeholderText: I18n.tr("Search wallhaven.cc…")
									placeholderTextColor: ThemeManager.onSurfaceVariant
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									leftPadding: 10; rightPadding: 10
									background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
															border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
									onAccepted: WallhavenService.search(text, _whSort.cur)
								}
								SettingBtn { label: I18n.tr("Search"); onClicked: WallhavenService.search(_whSearch.text, _whSort.cur) }
							}
							// Sorting
							RowLayout {
								Layout.fillWidth: true
								spacing: 4
								Repeater {
									model: [ { k: "toplist", l: I18n.tr("Top") }, { k: "date_added", l: I18n.tr("Latest") }, { k: "views", l: I18n.tr("Views") }, { k: "random", l: I18n.tr("Random") } ]
									delegate: Rectangle {
										required property var modelData
										readonly property bool sel: _whSort.cur === modelData.k
										implicitWidth: _whsl.implicitWidth + 18; implicitHeight: 26
										radius: ThemeManager.chipRadius
										color: sel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.18)
													: (_whsMa.containsMouse ? Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.08) : "transparent")
										Text { id: _whsl; anchors.centerIn: parent; text: modelData.l
												color: parent.sel ? ThemeManager.primary : ThemeManager.onSurface
												font.family: ThemeManager.fontFamily; font.pixelSize: 11 }
										MouseArea { id: _whsMa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
													onClicked: { _whSort.cur = modelData.k; WallhavenService.search(_whSearch.text, modelData.k) } }
									}
								}
								Item { Layout.fillWidth: true }
								Item {
									property string cur: "toplist"
									id: _whSort
								}
							}

							Text {
								visible: !WallpaperService.canDownload
								Layout.fillWidth: true
								text: I18n.tr("Browsing works, but downloading a wallpaper needs curl (install it to set/favorite from here).")
								wrapMode: Text.WordWrap; color: ThemeManager.error
								font.family: ThemeManager.fontFamily; font.pixelSize: 10
							}
							Text {
								visible: WallhavenService.error !== ""
								Layout.fillWidth: true
								text: WallhavenService.error
								color: ThemeManager.error; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
							}
							Text {
								visible: WallhavenService.loading && WallhavenService.results.length === 0
								text: I18n.tr("Searching…"); color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; opacity: 0.7
							}
							Text {
								visible: !WallhavenService.loading && WallhavenService.error === "" && WallhavenService.results.length === 0
								text: I18n.tr("Search wallhaven.cc for wallpapers."); color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; opacity: 0.7
							}

							Flow {
								Layout.fillWidth: true
								spacing: 8
								Repeater {
									model: root._wpTab === "browse" ? WallhavenService.results : []
									delegate: ClippingRectangle {
										id: _rtile
										required property var modelData
										readonly property bool _busy: WallpaperService.downloadingId === ("" + modelData.id)
										width: 168; height: 96
										radius: ThemeManager.chipRadius
										color: ThemeManager.surfaceContainerHigh

										Image {
											anchors.fill: parent
											source: _rtile.modelData.thumb
											fillMode: Image.PreserveAspectCrop
											asynchronous: true; cache: true
										}
										Rectangle {
											anchors.fill: parent; radius: _rtile.radius
											color: Qt.rgba(0, 0, 0, (_rtMa.containsMouse || _rtile._busy) ? 0.3 : 0)
										}
										// Resolution chip
										Text {
											anchors { bottom: parent.bottom; left: parent.left; margins: 4 }
											text: _rtile.modelData.resolution || ""
											color: "white"; style: Text.Outline; styleColor: "black"
											font.family: ThemeManager.fontFamily; font.pixelSize: 9
										}
										Text {
											anchors.centerIn: parent
											visible: _rtile._busy
											text: "󰇚"; color: "white"
											font.family: ThemeManager.fontFamily; font.pixelSize: 22
										}
										MouseArea {
											id: _rtMa; anchors.fill: parent
											hoverEnabled: true; cursorShape: Qt.PointingHandCursor
											enabled: WallpaperService.canDownload && !_rtile._busy
											// Click = download + set
											onClicked: WallpaperService.download(_rtile.modelData.full, _rtile.modelData.id, _rtile.modelData.fileType, false)
										}
										// Favorite (download + favorite) — above the tile MouseArea so it stays clickable
										WpTileBtn {
											anchors { top: parent.top; right: parent.right; margins: 5 }
											icon: "󰋕"
											onClicked: WallpaperService.download(_rtile.modelData.full, _rtile.modelData.id, _rtile.modelData.fileType, true)
										}
									}
								}
							}
							SettingBtn {
								Layout.topMargin: 6
								visible: WallhavenService.hasMore && !WallhavenService.loading
								label: I18n.tr("Load more")
								onClicked: WallhavenService.loadMore()
							}
							SettingText {
								Layout.topMargin: 10
								label: I18n.tr("Wallhaven API key")
								sub: I18n.tr("Optional — only needed to browse NSFW results")
								path: "wallpaper.wallhavenKey"
								placeholder: I18n.tr("from wallhaven.cc/settings/account")
							}
						}

						// ── Rotation ──────────────────────────────────────────────
						SettingSection { text: I18n.tr("Rotation"); Layout.topMargin: 14 }
						SettingRowBase {
							label: I18n.tr("Rotate wallpapers")
							sub: WallpaperService.rotationPaths.length + " selected — Ctrl-click a wallpaper to add, Shift-click for a range"
							Rectangle {
								implicitWidth: 40; implicitHeight: 22; radius: 11
								opacity: WallpaperService.rotationPaths.length > 1 ? 1 : 0.4
								color: WallpaperService.rotationEnabled ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
								Behavior on color { ColorAnimation { duration: 120 } }
								Rectangle {
									width: 16; height: 16; radius: 8; y: 3
									x: WallpaperService.rotationEnabled ? parent.width - width - 3 : 3
									color: WallpaperService.rotationEnabled ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
									Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
								}
								TapHandler {
									enabled: WallpaperService.rotationPaths.length > 1
									onTapped: WallpaperService.setRotationEnabled(!WallpaperService.rotationEnabled)
								}
							}
						}
						SettingRowBase {
							label: I18n.tr("Interval")
							Text {
								text: WallpaperService.rotationIntervalMin + " min"
								color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; Layout.rightMargin: 8
							}
							Rectangle {
								id: _wpIvTrack
								Layout.preferredWidth: 160; implicitHeight: 6; radius: 3
								color: ThemeManager.surfaceContainerHigh
								readonly property int _min: 1
								readonly property int _max: 120
								readonly property real _frac: Math.max(0, Math.min(1, (WallpaperService.rotationIntervalMin - _min) / (_max - _min)))
								Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
											width: _wpIvTrack.width * _wpIvTrack._frac; radius: 3; color: ThemeManager.primary }
								Rectangle { width: 14; height: 14; radius: 7; color: ThemeManager.primary
											y: -4; x: Math.max(0, Math.min(_wpIvTrack.width - width, _wpIvTrack.width * _wpIvTrack._frac - width / 2)) }
								MouseArea {
									anchors.fill: parent; anchors.margins: -6
									onPressed: (e) => _set(e.x); onPositionChanged: (e) => { if (pressed) _set(e.x) }
									function _set(x) {
										const f = Math.max(0, Math.min(1, (x - 6) / _wpIvTrack.width))
										WallpaperService.setRotationInterval(_wpIvTrack._min + f * (_wpIvTrack._max - _wpIvTrack._min))
									}
								}
							}
						}
						Text {
							Layout.fillWidth: true; Layout.topMargin: 2
							text: I18n.tr("Each change re-generates the Material You theme from the new wallpaper.")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10; opacity: 0.7
						}
					}

					// Hyprland ------------------------------------------------------
					ColumnLayout {
						id: _hlPane
						visible: SettingsUi.category === "hyprland"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6

						SettingSection { text: I18n.tr("Display settings") }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 4
							text: I18n.tr("Configure displays, image quality, input and window appearance. Changes are saved without replacing your original compositor configuration.")
							wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}

						// ── Sub-tabs ──────────────────────────────────────────────
						RowLayout {
							Layout.fillWidth: true
							spacing: 6
							Repeater {
								model: [ { id: "display", label: I18n.tr("Display") }, { id: "appearance", label: I18n.tr("Appearance") }, { id: "input", label: I18n.tr("Input") } ]
								delegate: Rectangle {
									required property var modelData
									readonly property bool sel: root._hlTab === modelData.id
									implicitWidth: _htl.implicitWidth + 24; implicitHeight: 30
									radius: ThemeManager.chipRadius
									color: sel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.18)
												: (_htMa.containsMouse ? Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.08) : "transparent")
									Text {
										id: _htl; anchors.centerIn: parent; text: modelData.label
										color: parent.sel ? ThemeManager.primary : ThemeManager.onSurface
										font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
									}
									MouseArea { id: _htMa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root._hlTab = modelData.id }
								}
							}
							Item { Layout.fillWidth: true }
							SettingBtn { label: I18n.tr("Refresh"); onClicked: HyprlandConfigService.refresh() }
						}

						// ── Display tab ───────────────────────────────────────────
						ColumnLayout {
							id: _dispTab
							visible: root._hlTab === "display"
							Layout.fillWidth: true
							Layout.topMargin: 6
							spacing: 8

							property string sel: ""
							property bool _detModeOpen: false
							// Selection with a fallback to the first monitor so the detail
							// panel always has a target (probe may finish after onCompleted).
							readonly property string effSel: {
								const ms = HyprlandConfigService.monitors
								if (sel !== "") for (const m of ms) if (m.name === sel) return sel
								return ms.length ? ms[0].name : ""
							}

							function mget(name, k, d) { return SettingsService.get("hypr.monitors." + name + "." + k, d) }
							function selMon() { for (const m of HyprlandConfigService.monitors) if (m.name === _dispTab.effSel) return m; return null }

							// Effective logical geometry for a monitor (staged override or live).
							// Logical size = mode pixels / scale, swapped for 90°/270° rotation.
							function mgeo(m) {
								void SettingsService.rev
								const modeStr = mget(m.name, "mode", m.width + "x" + m.height + "@" + Number(m.refreshRate).toFixed(2))
								const mm = ("" + modeStr).split("@")[0].split("x")
								let W = parseInt(mm[0]) || m.width, H = parseInt(mm[1]) || m.height
								const sc = parseFloat(mget(m.name, "scale", m.scale)) || 1
								const tr = parseInt(mget(m.name, "transform", m.transform)) || 0
								if (tr === 1 || tr === 3) { const t = W; W = H; H = t }
								return {
									name: m.name,
									on:   mget(m.name, "enabled", !m.disabled),
									lx:   parseInt(mget(m.name, "x", m.x)) || 0,
									ly:   parseInt(mget(m.name, "y", m.y)) || 0,
									lw:   Math.max(1, Math.round(W / sc)),
									lh:   Math.max(1, Math.round(H / sc))
								}
							}

							Text {
								Layout.fillWidth: true
								text: I18n.tr("Drag a screen to reposition it; it snaps to its neighbours' edges. Click to select and edit its settings below.")
								wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: 10; opacity: 0.7
							}

							// ── Visual layout canvas ──────────────────────────────────
							Rectangle {
								id: _canvas
								Layout.fillWidth: true
								Layout.preferredHeight: 260
								radius: ThemeManager.chipRadius
								color: ThemeManager.surfaceContainerLow
								border.width: 1; border.color: ThemeManager.outlineVariant
								clip: true

								readonly property var _boxes: { void SettingsService.rev; return HyprlandConfigService.monitors.map(m => _dispTab.mgeo(m)) }
								readonly property real _pad: 18
								readonly property real _minX: _boxes.length ? Math.min(..._boxes.map(b => b.lx)) : 0
								readonly property real _minY: _boxes.length ? Math.min(..._boxes.map(b => b.ly)) : 0
								readonly property real _spanX: _boxes.length ? Math.max(1, Math.max(..._boxes.map(b => b.lx + b.lw)) - _minX) : 1
								readonly property real _spanY: _boxes.length ? Math.max(1, Math.max(..._boxes.map(b => b.ly + b.lh)) - _minY) : 1
								readonly property real k: Math.min((width - 2 * _pad) / _spanX, (height - 2 * _pad) / _spanY)
								readonly property real _offX: (width  - _spanX * k) / 2
								readonly property real _offY: (height - _spanY * k) / 2
								function px(lx) { return _offX + (lx - _minX) * k }
								function py(ly) { return _offY + (ly - _minY) * k }

								// Deselect when clicking empty canvas
								MouseArea { anchors.fill: parent; onClicked: {} }

								Repeater {
									model: HyprlandConfigService.monitors
									delegate: Rectangle {
										id: _mtile
										required property var modelData
										readonly property string _mn: modelData.name
										readonly property var b: _dispTab.mgeo(modelData)
										readonly property bool _isSel: _dispTab.effSel === _mn
										property real _grabX: 0
										property real _grabY: 0
										property int  _origX: 0
										property int  _origY: 0
										property bool _moved: false

										x: _canvas.px(b.lx);  y: _canvas.py(b.ly)
										width:  Math.max(24, b.lw * _canvas.k)
										height: Math.max(18, b.lh * _canvas.k)
										radius: 6
										opacity: b.on ? 1 : 0.45
										color: _isSel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.28)
														: ThemeManager.surfaceContainerHigh
										border.width: _isSel ? 2 : 1
										border.color: _isSel ? ThemeManager.primary : ThemeManager.outlineVariant

										Column {
											anchors.centerIn: parent
											spacing: 1
											Text {
												anchors.horizontalCenter: parent.horizontalCenter
												text: _mtile._mn
												color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
												font.pixelSize: ThemeManager.fontSizeSm; font.bold: true
											}
											Text {
												anchors.horizontalCenter: parent.horizontalCenter
												visible: _mtile.height > 34
												text: _mtile.b.on ? (_mtile.b.lw + "×" + _mtile.b.lh) : "off"
												color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9
											}
										}

										MouseArea {
											anchors.fill: parent
											cursorShape: Qt.OpenHandCursor
											onPressed: (m) => {
												_dispTab.sel = _mtile._mn
												_mtile._moved = false
												const p = mapToItem(_canvas, m.x, m.y)
												_mtile._grabX = p.x; _mtile._grabY = p.y
												_mtile._origX = _mtile.b.lx; _mtile._origY = _mtile.b.ly
											}
											onPositionChanged: (m) => {
												if (!pressed) return
												const p = mapToItem(_canvas, m.x, m.y)
												if (!_mtile._moved && Math.abs(p.x - _mtile._grabX) + Math.abs(p.y - _mtile._grabY) < 3) return
												_mtile._moved = true
												const nx = Math.round(_mtile._origX + (p.x - _mtile._grabX) / _canvas.k)
												const ny = Math.round(_mtile._origY + (p.y - _mtile._grabY) / _canvas.k)
												HyprlandConfigService.stageMonitor(_mtile._mn, "x", nx)
												HyprlandConfigService.stageMonitor(_mtile._mn, "y", ny)
											}
											onReleased: { if (_mtile._moved) _dispTab.snap(_mtile._mn) }
										}
									}
								}
							}

							// Snap the given monitor's edges to its neighbours (kill gaps/overlap).
							function snap(name) {
								const me = mgeo(selMonBy(name))
								if (!me) return
								const others = HyprlandConfigService.monitors.filter(m => m.name !== name).map(m => mgeo(m))
								const th = Math.max(40, _canvas._spanX * 0.05)
								let nx = me.lx, ny = me.ly
								for (const o of others) {
									// horizontal edge snapping
									if (Math.abs((me.lx + me.lw) - o.lx) < th) nx = o.lx - me.lw
									else if (Math.abs(me.lx - (o.lx + o.lw)) < th) nx = o.lx + o.lw
									else if (Math.abs(me.lx - o.lx) < th) nx = o.lx
									// vertical edge / top-align snapping
									if (Math.abs((me.ly + me.lh) - o.ly) < th) ny = o.ly - me.lh
									else if (Math.abs(me.ly - (o.ly + o.lh)) < th) ny = o.ly + o.lh
									else if (Math.abs(me.ly - o.ly) < th) ny = o.ly
								}
								if (nx !== me.lx) HyprlandConfigService.stageMonitor(name, "x", nx)
								if (ny !== me.ly) HyprlandConfigService.stageMonitor(name, "y", ny)
							}
							function selMonBy(name) { for (const m of HyprlandConfigService.monitors) if (m.name === name) return m; return null }

							// ── Selected-monitor detail ───────────────────────────────
							Rectangle {
								Layout.fillWidth: true
								Layout.topMargin: 4
								visible: _dispTab.selMon() !== null
								implicitHeight: _detCol.implicitHeight + 24
								radius: ThemeManager.chipRadius
								color: ThemeManager.surfaceContainerLow
								border.width: 1; border.color: ThemeManager.outlineVariant

								readonly property var m: _dispTab.selMon()
								readonly property string _mn: m ? m.name : ""
								readonly property var _hdrCap: m ? HyprlandConfigService.hdrCapability(_mn) : null
								readonly property bool _hdrSupported: !!_hdrCap && !!_hdrCap.hdr && _hdrCap.bitDepth >= 10
								function _get(k, d) { return m ? SettingsService.get("hypr.monitors." + _mn + "." + k, d) : d }
								readonly property bool _on: m ? _get("enabled", !m.disabled) : true
								readonly property string _hdrMode: {
									if (!m) return "off"
									const saved = _get("hdrMode", "")
									if (saved === "auto" || saved === "always" || saved === "off") return saved
									const legacy = SettingsService.get("hypr.monitors." + _mn + ".hdr", undefined)
									return legacy === true ? "always" : "off"
								}
								readonly property bool _hdrOn: _hdrMode !== "off"
								id: _det

								ColumnLayout {
									id: _detCol
									anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
									spacing: 8

									RowLayout {
										Layout.fillWidth: true; spacing: 8
										Text {
											text: _det._mn
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
											font.pixelSize: ThemeManager.fontSizeMd; font.bold: true
										}
										Text {
											Layout.fillWidth: true
											text: _det.m ? (_det.m.width + "×" + _det.m.height + " @" + Number(_det.m.refreshRate).toFixed(0) + "Hz") : ""
											color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10
											elide: Text.ElideRight
										}
										Text { text: I18n.tr("On"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
										Rectangle {
											implicitWidth: 40; implicitHeight: 22; radius: 11
											color: _det._on ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
											Rectangle { width: 16; height: 16; radius: 8; y: 3; x: _det._on ? parent.width - width - 3 : 3
														color: _det._on ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
														Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } } }
											TapHandler { onTapped: HyprlandConfigService.stageMonitor(_det._mn, "enabled", !_det._on) }
										}
									}

									// Resolution (inline expanding)
									ColumnLayout {
										visible: _det._on
										Layout.fillWidth: true
										spacing: 4
										RowLayout {
											Layout.fillWidth: true; spacing: 8
											Text { text: I18n.tr("Resolution"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
											Item { Layout.fillWidth: true }
											Rectangle {
												implicitWidth: _detRes.implicitWidth + 26; implicitHeight: 28
												radius: ThemeManager.chipRadius
												color: ThemeManager.surfaceContainerHigh
												border.width: 1; border.color: _dispTab._detModeOpen ? ThemeManager.primary : ThemeManager.outlineVariant
												Text {
													id: _detRes; anchors.centerIn: parent
													text: (_det.m ? _det._get("mode", _det.m.width + "x" + _det.m.height + "@" + Number(_det.m.refreshRate).toFixed(2)) : "") + "  ▾"
													color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
												}
												MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: _dispTab._detModeOpen = !_dispTab._detModeOpen }
											}
										}
										Flow {
											visible: _dispTab._detModeOpen
											Layout.fillWidth: true
											spacing: 4
											Repeater {
												model: (_dispTab._detModeOpen && _det.m) ? _det.m.modes : []
												delegate: Rectangle {
													required property var modelData
													readonly property bool sel: _det._get("mode", "") === modelData
													implicitWidth: _dmo.implicitWidth + 16; implicitHeight: 24
													radius: ThemeManager.chipRadius
													color: sel ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.18) : ThemeManager.surfaceContainerHigh
													border.width: 1; border.color: ThemeManager.outlineVariant
													Text { id: _dmo; anchors.centerIn: parent; text: modelData
															color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
													MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor
																onClicked: { HyprlandConfigService.stageMonitor(_det._mn, "mode", modelData); _dispTab._detModeOpen = false } }
												}
											}
										}
									}

									// Scale + position + rotation
									GridLayout {
										visible: _det._on
										Layout.fillWidth: true
										columns: 2; columnSpacing: 12; rowSpacing: 6

										Text { text: I18n.tr("Scale"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
										TextField {
											Layout.preferredWidth: 90; implicitHeight: 28
											text: _det.m ? "" + _det._get("scale", _det.m.scale) : ""
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
											leftPadding: 8; rightPadding: 8
											background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
																	border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
											onEditingFinished: { const v = parseFloat(text); if (!isNaN(v)) HyprlandConfigService.stageMonitor(_det._mn, "scale", v) }
										}

										Text { text: I18n.tr("Position (x, y)"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
										RowLayout {
											spacing: 6
											TextField {
												Layout.preferredWidth: 70; implicitHeight: 28
												text: _det.m ? "" + _det._get("x", _det.m.x) : ""
												color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
												leftPadding: 8; rightPadding: 8
												background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
																		border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
												onEditingFinished: { const v = parseInt(text); if (!isNaN(v)) HyprlandConfigService.stageMonitor(_det._mn, "x", v) }
											}
											TextField {
												Layout.preferredWidth: 70; implicitHeight: 28
												text: _det.m ? "" + _det._get("y", _det.m.y) : ""
												color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
												leftPadding: 8; rightPadding: 8
												background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
																		border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
												onEditingFinished: { const v = parseInt(text); if (!isNaN(v)) HyprlandConfigService.stageMonitor(_det._mn, "y", v) }
											}
										}

										Text { text: I18n.tr("Rotation"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
										Row {
											spacing: 0
											Repeater {
												model: [ "0°", "90°", "180°", "270°" ]
												delegate: Rectangle {
													required property var modelData
													required property int index
													readonly property bool sel: _det.m ? (_det._get("transform", _det.m.transform) === index) : false
													implicitWidth: _dtr.implicitWidth + 18; implicitHeight: 26
													color: sel ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
													border.width: 1; border.color: ThemeManager.outlineVariant
													Text { id: _dtr; anchors.centerIn: parent; text: modelData
															color: sel ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
															font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
													TapHandler { onTapped: HyprlandConfigService.stageMonitor(_det._mn, "transform", index) }
												}
											}
										}
									}

									// HDR is deliberately capability-gated using the monitor's
									// EDID. Applying it participates in the same 15-second safe
									// confirmation flow as resolution and layout changes.
									Rectangle {
										visible: _det._on
										Layout.fillWidth: true
										implicitHeight: _hdrCol.implicitHeight + 20
										radius: ThemeManager.chipRadius
										color: ThemeManager.surfaceContainerHigh
										border.width: 1
										border.color: _det._hdrOn ? ThemeManager.primary : ThemeManager.outlineVariant

										ColumnLayout {
											id: _hdrCol
											anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
											spacing: 8

											RowLayout {
												Layout.fillWidth: true; spacing: 8
												Text {
													text: "HDR"
													color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
													font.pixelSize: ThemeManager.fontSizeMd; font.bold: true
												}
												Rectangle {
													implicitWidth: _hdrBadge.implicitWidth + 14; implicitHeight: 20; radius: 10
													color: _det._hdrSupported
														? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.16)
														: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.07)
													Text {
														id: _hdrBadge; anchors.centerIn: parent
														text: _det._hdrSupported
															? I18n.tr("Compatible") + " · " + _det._hdrCap.bitDepth + " bit"
																+ (_det._hdrCap.maxLuminance > 0 ? " · " + Math.round(_det._hdrCap.maxLuminance) + " nits" : "")
															: I18n.tr("Not supported by this display")
														color: _det._hdrSupported ? ThemeManager.primary : ThemeManager.onSurfaceVariant
														font.family: ThemeManager.fontFamily; font.pixelSize: 10
													}
												}
												Item { Layout.fillWidth: true }
											}

											RowLayout {
												Layout.fillWidth: true
												spacing: 0
												Repeater {
													model: [
														{ label: I18n.tr("Automatic"), key: "auto" },
														{ label: I18n.tr("Always on"), key: "always" },
														{ label: I18n.tr("Off"), key: "off" }
													]
													delegate: Rectangle {
														required property var modelData
														implicitWidth: _hdrModeText.implicitWidth + 24
														implicitHeight: 28
														opacity: _det._hdrSupported ? 1 : 0.4
														color: _det._hdrMode === modelData.key
															? ThemeManager.primary : ThemeManager.surfaceContainerLow
														border.width: 1
														border.color: ThemeManager.outlineVariant
														Text {
															id: _hdrModeText
															anchors.centerIn: parent
															text: modelData.label
															color: _det._hdrMode === modelData.key
																? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
															font.family: ThemeManager.fontFamily
															font.pixelSize: ThemeManager.fontSizeSm
														}
														TapHandler {
															enabled: _det._hdrSupported
															onTapped: {
																if (modelData.key !== "off"
																		&& SettingsService.get("hypr.monitors." + _det._mn + ".sdrBrightness", undefined) === undefined) {
																	HyprlandConfigService.stageMonitor(_det._mn, "sdrBrightness", 1.5)
																	HyprlandConfigService.stageMonitor(_det._mn, "sdrSaturation", 1.0)
																}
																HyprlandConfigService.stageHdrMode(_det._mn, modelData.key)
															}
														}
													}
												}
												Item { Layout.fillWidth: true }
											}

											Text {
												Layout.fillWidth: true
												text: !_det._hdrSupported
													? I18n.tr("HDR is shown only when the connected display reports PQ HDR and 10-bit color support.")
													: (_det._hdrMode === "auto"
														? I18n.tr("Keeps a 10-bit link and activates HDR only when a fullscreen app provides HDR content. The desktop returns to accurate SDR automatically.")
														: (_det._hdrMode === "always"
															? I18n.tr("Forces HDR output at all times, including the desktop.")
															: I18n.tr("Uses an 8-bit SDR link and never enables HDR.")))
												wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
												font.family: ThemeManager.fontFamily; font.pixelSize: 10
											}

											ColumnLayout {
												visible: _det._hdrSupported && _det._hdrOn
												Layout.fillWidth: true; spacing: 6
												SettingSection { text: I18n.tr("HDR calibration") }
												Text {
													Layout.fillWidth: true
													text: I18n.tr("Adjust how ordinary SDR applications look while HDR output is active. HDR videos and games keep their own luminance metadata.")
													wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
													font.family: ThemeManager.fontFamily; font.pixelSize: 10
												}
												HdrSlider {
													label: I18n.tr("SDR brightness")
													sub: I18n.tr("Raise it if the desktop looks too dark")
													monitorName: _det._mn; keyName: "sdrBrightness"
													def: 1.5
													from: 0.8; to: 2.0; step: 0.05; decimals: 2; unit: "×"
												}
												HdrSlider {
													label: I18n.tr("SDR saturation")
													sub: I18n.tr("Keep near 1.00 for accurate colors")
													monitorName: _det._mn; keyName: "sdrSaturation"
													def: _det.m ? _det.m.sdrSaturation : 1.0
													from: 0.8; to: 1.2; step: 0.01; decimals: 2; unit: "×"
												}
												RowLayout {
													Layout.fillWidth: true; spacing: 6
													Text {
														text: I18n.tr("Presets")
														color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily
														font.pixelSize: ThemeManager.fontSizeSm
													}
													Item { Layout.fillWidth: true }
													SettingBtn {
														label: I18n.tr("Soft")
														onClicked: {
															HyprlandConfigService.stageMonitor(_det._mn, "sdrBrightness", 1.25)
															HyprlandConfigService.stageMonitor(_det._mn, "sdrSaturation", 1.0)
														}
													}
													SettingBtn {
														label: I18n.tr("Balanced")
														onClicked: {
															HyprlandConfigService.stageMonitor(_det._mn, "sdrBrightness", 1.5)
															HyprlandConfigService.stageMonitor(_det._mn, "sdrSaturation", 1.0)
														}
													}
													SettingBtn {
														label: I18n.tr("Bright")
														onClicked: {
															HyprlandConfigService.stageMonitor(_det._mn, "sdrBrightness", 1.8)
															HyprlandConfigService.stageMonitor(_det._mn, "sdrSaturation", 1.02)
														}
													}
												}
											}
										}
									}

									SettingBtn {
										label: I18n.tr("Reset this monitor"); danger: true
										onClicked: HyprlandConfigService.resetMonitor(_det._mn)
									}
								}
							}

							RowLayout {
								Layout.fillWidth: true
								Layout.topMargin: 4
								spacing: 10
								SettingBtn {
									label: I18n.tr("Apply display changes")
									enabled: HyprlandConfigService.monitorsDirty
									onClicked: HyprlandConfigService.applyMonitors()
								}
								Text {
									Layout.fillWidth: true
									text: I18n.tr("Display changes ask for confirmation and auto-revert after 15 s if not kept.")
									wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant
									font.family: ThemeManager.fontFamily; font.pixelSize: 10; opacity: 0.7
								}
							}
						}

						// ── Appearance tab ────────────────────────────────────────
						ColumnLayout {
							visible: root._hlTab === "appearance"
							Layout.fillWidth: true
							Layout.topMargin: 6
							spacing: 6

							SettingSection { text: I18n.tr("Gaps & borders") }
							SettingSlider { label: I18n.tr("Gaps in");  path: "hypr.general.gaps_in";  def: HyprlandConfigService.live("general:gaps_in", 5);   from: 0; to: 40; unit: "px"; applyFn: () => HyprlandConfigService.applyLive() }
							SettingSlider { label: I18n.tr("Gaps out"); path: "hypr.general.gaps_out"; def: HyprlandConfigService.live("general:gaps_out", 20); from: 0; to: 60; unit: "px"; applyFn: () => HyprlandConfigService.applyLive() }
							SettingSlider { label: I18n.tr("Border size"); path: "hypr.general.border_size"; def: HyprlandConfigService.live("general:border_size", 2); from: 0; to: 8; unit: "px"; applyFn: () => HyprlandConfigService.applyLive() }
							SettingText   { label: I18n.tr("Active border");   path: "hypr.general.col.active_border";   def: ""; placeholder: I18n.tr("rgba(4fcf8cee)"); applyFn: () => HyprlandConfigService.applyLive() }
							SettingText   { label: I18n.tr("Inactive border"); path: "hypr.general.col.inactive_border"; def: ""; placeholder: I18n.tr("rgba(595959aa)"); applyFn: () => HyprlandConfigService.applyLive() }

							SettingSection { text: I18n.tr("Decoration") }
							SettingSlider { label: I18n.tr("Rounding"); path: "hypr.decoration.rounding"; def: HyprlandConfigService.live("decoration:rounding", 10); from: 0; to: 24; unit: "px"; applyFn: () => HyprlandConfigService.applyLive() }
							SettingToggle { label: I18n.tr("Blur"); path: "hypr.decoration.blur.enabled"; def: HyprlandConfigService.live("decoration:blur:enabled", true); applyFn: () => HyprlandConfigService.applyLive() }
							SettingSlider { label: I18n.tr("Blur size");   path: "hypr.decoration.blur.size";   def: HyprlandConfigService.live("decoration:blur:size", 8);   from: 1; to: 20; unit: ""; applyFn: () => HyprlandConfigService.applyLive() }
							SettingSlider { label: I18n.tr("Blur passes"); path: "hypr.decoration.blur.passes"; def: HyprlandConfigService.live("decoration:blur:passes", 3); from: 1; to: 6;  unit: ""; applyFn: () => HyprlandConfigService.applyLive() }
							SettingToggle { label: I18n.tr("Shadow"); path: "hypr.decoration.shadow.enabled"; def: HyprlandConfigService.live("decoration:shadow:enabled", true); applyFn: () => HyprlandConfigService.applyLive() }

							RowLayout {
								Layout.fillWidth: true; Layout.topMargin: 6
								Item { Layout.fillWidth: true }
								SettingBtn { label: I18n.tr("Reset to my config"); danger: true
											onClicked: HyprlandConfigService.resetKeys(["general", "decoration"]) }
							}
						}

						// ── Input tab ─────────────────────────────────────────────
						ColumnLayout {
							visible: root._hlTab === "input"
							Layout.fillWidth: true
							Layout.topMargin: 6
							spacing: 6

							SettingSection { text: I18n.tr("Keyboard") }
							SettingText { label: I18n.tr("Layout");  path: "hypr.input.kb_layout";  def: ""; placeholder: HyprlandConfigService.live("input:kb_layout", "us"); applyFn: () => HyprlandConfigService.applyLive() }
							SettingText { label: I18n.tr("Variant"); path: "hypr.input.kb_variant"; def: ""; placeholder: HyprlandConfigService.live("input:kb_variant", "—"); applyFn: () => HyprlandConfigService.applyLive() }

							SettingSection { text: I18n.tr("Mouse & touchpad") }
							SettingText   { label: I18n.tr("Sensitivity"); sub: I18n.tr("−1.0 to 1.0"); path: "hypr.input.sensitivity"; def: ""; placeholder: "" + HyprlandConfigService.live("input:sensitivity", 0); applyFn: () => HyprlandConfigService.applyLive() }
							SettingSeg    { label: I18n.tr("Follow mouse"); path: "hypr.input.follow_mouse"; def: "" + HyprlandConfigService.live("input:follow_mouse", 1)
											options: [ "0", "1", "2", "3" ]; keys: [ "0", "1", "2", "3" ]; applyFn: () => HyprlandConfigService.applyLive() }
							SettingToggle { label: I18n.tr("Touchpad natural scroll"); path: "hypr.input.touchpad.natural_scroll"; def: HyprlandConfigService.live("input:touchpad:natural_scroll", false); applyFn: () => HyprlandConfigService.applyLive() }

							SettingSection { text: I18n.tr("Behavior") }
							SettingSeg    { label: I18n.tr("Layout"); path: "hypr.general.layout"; def: HyprlandConfigService.live("general:layout", "dwindle")
											options: [ "Dwindle", "Master" ]; keys: [ "dwindle", "master" ]; applyFn: () => HyprlandConfigService.applyLive() }
							SettingToggle { label: I18n.tr("Allow tearing"); sub: I18n.tr("For fullscreen games"); path: "hypr.general.allow_tearing"; def: HyprlandConfigService.live("general:allow_tearing", false); applyFn: () => HyprlandConfigService.applyLive() }
							SettingToggle { label: I18n.tr("Animations"); path: "hypr.animations.enabled"; def: HyprlandConfigService.live("animations:enabled", true); applyFn: () => HyprlandConfigService.applyLive() }

							RowLayout {
								Layout.fillWidth: true; Layout.topMargin: 6
								Item { Layout.fillWidth: true }
								SettingBtn { label: I18n.tr("Reset to my config"); danger: true
											onClicked: HyprlandConfigService.resetKeys(["input", "general.layout", "general.allow_tearing", "animations"]) }
							}
						}
					}

					// Storage -------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "storage"
						Layout.fillWidth: true; Layout.margins: 20; spacing: 12
						SettingSection { text: I18n.tr("Storage") }
						RowLayout {
							Layout.fillWidth: true
							Text { Layout.fillWidth: true; text: StorageService.formatBytes(StorageService.usedBytes) + " " + I18n.tr("used of") + " " + StorageService.formatBytes(StorageService.totalBytes); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeMd; font.bold: true }
							SettingBtn { label: StorageService.loading ? I18n.tr("Calculating…") : I18n.tr("Refresh"); enabled: !StorageService.loading; onClicked: StorageService.refresh() }
						}
						Rectangle {
							Layout.fillWidth: true; implicitHeight: 14; radius: 7; color: ThemeManager.surfaceContainerHigh; clip: true
							Row {
								anchors.fill: parent
								Repeater {
									model: StorageService.categories
									Rectangle {
										required property var modelData
										required property int index
										height: parent.height
										width: StorageService.usedBytes > 0 ? parent.width * modelData.bytes / StorageService.usedBytes : 0
										color: [ThemeManager.primary, ThemeManager.tertiary, ThemeManager.secondary, "#79c7ff", "#ffb4ab", ThemeManager.outline][index % 6]
									}
								}
							}
						}
						Repeater {
							model: StorageService.categories
							delegate: Rectangle {
								required property var modelData
								required property int index
								Layout.fillWidth: true; implicitHeight: 58; radius: ThemeManager.chipRadius + 2
								color: _storageHover.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow
								border.width: 1; border.color: ThemeManager.outlineVariant
								RowLayout {
									anchors { fill: parent; margins: 12 }
									spacing: 10
									Rectangle { width: 10; height: 10; radius: 5; color: [ThemeManager.primary, ThemeManager.tertiary, ThemeManager.secondary, "#79c7ff", "#ffb4ab", ThemeManager.outline][index % 6] }
									Text { Layout.fillWidth: true; text: I18n.tr(modelData.key); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeMd }
									Text { text: StorageService.formatBytes(modelData.bytes); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
									Text { visible: modelData.key === "Applications"; text: "󰅂"; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily }
								}
								HoverHandler { id: _storageHover }
								TapHandler { enabled: modelData.key === "Applications"; onTapped: { StorageService.loadApps(); SettingsUi.category = "storage-apps" } }
							}
						}
					}

					ColumnLayout {
						visible: SettingsUi.category === "storage-apps"
						Layout.fillWidth: true; Layout.margins: 20; spacing: 8
						SettingSection { text: I18n.tr("Installed applications") }
						TextField {
							Layout.fillWidth: true; implicitHeight: 36
							placeholderText: I18n.tr("Search applications…")
							placeholderTextColor: ThemeManager.onSurfaceVariant
							onTextChanged: root._storageSearch = text.toLowerCase()
							color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
							background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh; border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
						}
						Text { visible: StorageService.appsLoading; text: I18n.tr("Loading applications…"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily }
						Text { visible: StorageService.statusText !== ""; text: StorageService.statusText; color: StorageService.passwordError ? ThemeManager.error : ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
						Repeater {
							model: StorageService.apps.filter(a => root._storageSearch === "" || a.name.toLowerCase().includes(root._storageSearch) || a.id.toLowerCase().includes(root._storageSearch))
							delegate: Rectangle {
								required property var modelData
								Layout.fillWidth: true; implicitHeight: 56; radius: ThemeManager.chipRadius + 2
								color: ThemeManager.surfaceContainerLow; border.width: 1; border.color: ThemeManager.outlineVariant
								RowLayout {
									anchors { fill: parent; margins: 10 }
									spacing: 10
									Rectangle {
										Layout.preferredWidth: 38
										Layout.preferredHeight: 38
										radius: 10
										color: ThemeManager.surfaceContainerHigh
										border.width: 1
										border.color: ThemeManager.outlineVariant
										IconImage {
											anchors.centerIn: parent
											implicitSize: 26
											source: (modelData.icon || "").startsWith("/")
												? modelData.icon
												: Quickshell.iconPath(modelData.icon || modelData.id, "application-x-executable")
										}
									}
									ColumnLayout {
										Layout.fillWidth: true; spacing: 0
										Text { text: modelData.name; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; elide: Text.ElideRight; Layout.fillWidth: true }
										Text { text: modelData.id + " · " + (modelData.kind === "pacman" ? StorageService.formatBytes(modelData.size) : modelData.size); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
									}
									SettingBtn { label: I18n.tr("Uninstall"); danger: true; onClicked: StorageService.requestUninstall(modelData.kind, modelData.id) }
								}
							}
						}
						Rectangle {
							visible: StorageService.passwordRequested
							Layout.fillWidth: true; implicitHeight: 118; radius: ThemeManager.panelRadius
							color: ThemeManager.surfaceContainerHigh; border.width: 1; border.color: StorageService.passwordError ? ThemeManager.error : ThemeManager.primary
							ColumnLayout {
								anchors { fill: parent; margins: 12 }
								spacing: 8
								Text { text: I18n.tr("Administrator password") + " · " + StorageService.pendingPackage; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.bold: true }
								TextField {
									id: _removePassword
									Layout.fillWidth: true
									echoMode: TextInput.Password
									placeholderText: I18n.tr("Password")
									placeholderTextColor: ThemeManager.onSurfaceVariant
									color: ThemeManager.onSurface
									font.family: ThemeManager.fontFamily
									background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer; border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
									onAccepted: { const p = text; text = ""; StorageService.submitPassword(p) }
								}
								RowLayout {
									Layout.fillWidth: true
									Item { Layout.fillWidth: true }
									SettingBtn { label: I18n.tr("Cancel"); onClicked: StorageService.cancelPassword() }
									SettingBtn { label: I18n.tr("Uninstall"); danger: true; onClicked: { const p = _removePassword.text; _removePassword.text = ""; StorageService.submitPassword(p) } }
								}
							}
						}
					}

					// Updates -------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "updates"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 10

						SettingSection { text: I18n.tr("System updates") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Update Arch, applications and firmware from one place. System updates always include the kernel to keep Arch consistent.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily
							font.pixelSize: ThemeManager.fontSizeSm
						}

						RowLayout {
							Layout.fillWidth: true
							Layout.topMargin: 4
							Text {
								Layout.fillWidth: true
								text: UpdateService.checking
									? I18n.tr("Checking for updates…")
									: (I18n.tr("Last checked") + ": " + (UpdateService.lastChecked || "—"))
								color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily
								font.pixelSize: 10
							}
							SettingBtn {
								label: I18n.tr("Check now")
								enabled: !UpdateService.checking
								onClicked: UpdateService.check()
							}
							SettingBtn { label: UpdateService.running ? I18n.tr("Updating…") : I18n.tr("Update everything"); enabled: !UpdateService.running; onClicked: UpdateService.request("all") }
						}

						Repeater {
							model: [
								{ key: "system", icon: "󰏖", title: I18n.tr("System and kernel"), detail: I18n.tr("Official Arch packages, dependencies and kernel"), count: UpdateService.systemUpdates },
								{ key: "aur", icon: "󰣇", title: "AUR", detail: I18n.tr("User repository applications"), count: UpdateService.aurUpdates },
								{ key: "flatpak", icon: "󰏗", title: "Flatpak", detail: I18n.tr("Sandboxed applications and runtimes"), count: UpdateService.flatpakUpdates },
								{ key: "firmware", icon: "󰒋", title: I18n.tr("Firmware"), detail: I18n.tr("Device firmware through fwupd"), count: -1 }
							]
							delegate: Rectangle {
								required property var modelData
								Layout.fillWidth: true
								implicitHeight: 64
								radius: ThemeManager.chipRadius + 2
								color: ThemeManager.surfaceContainerLow
								border.width: 1
								border.color: ThemeManager.outlineVariant
								RowLayout {
									anchors { fill: parent; margins: 10 }
									spacing: 10
									Text { text: modelData.icon; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 20 }
									ColumnLayout {
										Layout.fillWidth: true; spacing: 1
										Text { text: modelData.title; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeMd; font.weight: Font.Medium }
										Text { text: modelData.detail; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
									}
									Text {
										visible: modelData.count >= 0
										text: modelData.count + " " + I18n.tr(modelData.count === 1 ? "update" : "updates")
										color: modelData.count > 0 ? ThemeManager.primary : ThemeManager.onSurfaceVariant
										font.family: ThemeManager.fontFamily; font.pixelSize: 10
									}
									SettingBtn {
										label: I18n.tr("Update")
										enabled: !UpdateService.running && (modelData.key !== "firmware" || DependencyService.available("fwupdmgr"))
										onClicked: {
											UpdateService.request(modelData.key)
										}
									}
								}
							}
						}

						Text {
							visible: UpdateService.statusText !== ""
							Layout.fillWidth: true
							text: UpdateService.statusText
							color: UpdateService.running ? ThemeManager.primary : ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}

						SettingSection { text: I18n.tr("Automatic updates"); Layout.topMargin: 10 }
						Repeater {
							model: [
								{ key: "system", title: I18n.tr("System and kernel"), sub: I18n.tr("Install automatically every day"), enabled: UpdateService.autoSystem },
								{ key: "apps", title: I18n.tr("Applications"), sub: I18n.tr("Update AUR and Flatpak applications every day"), enabled: UpdateService.autoApps },
								{ key: "firmware", title: I18n.tr("Firmware"), sub: I18n.tr("Check and install firmware weekly"), enabled: UpdateService.autoFirmware }
							]
							delegate: SettingRowBase {
								required property var modelData
								label: modelData.title
								sub: modelData.sub
								Rectangle {
									implicitWidth: 40; implicitHeight: 22; radius: 11
									opacity: UpdateService.running ? 0.5 : 1
									color: modelData.enabled ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
									Rectangle {
										width: 16; height: 16; radius: 8; y: 3
										x: modelData.enabled ? parent.width - width - 3 : 3
										color: modelData.enabled ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
										Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
									}
									TapHandler { enabled: !UpdateService.running; onTapped: UpdateService.setAutomatic(modelData.key, !modelData.enabled) }
								}
							}
						}

						Rectangle {
							visible: UpdateService.passwordRequested
							Layout.fillWidth: true
							implicitHeight: _sudoCol.implicitHeight + 24
							radius: ThemeManager.panelRadius
							color: ThemeManager.surfaceContainerHigh
							border.width: 1; border.color: UpdateService.passwordError ? ThemeManager.error : ThemeManager.primary
							ColumnLayout {
								id: _sudoCol
								anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
								spacing: 8
								Text { text: I18n.tr("Administrator password"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeMd; font.bold: true }
								Text { text: I18n.tr("The password is used only to authorize this update and is never saved."); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10; wrapMode: Text.WordWrap; Layout.fillWidth: true }
								TextField {
									id: _sudoPassword
									Layout.fillWidth: true; implicitHeight: 34
									echoMode: TextInput.Password
									enabled: !UpdateService.authenticating
									placeholderText: I18n.tr("Password")
									placeholderTextColor: ThemeManager.onSurfaceVariant
									color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
									background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainer; border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
									onAccepted: { const value = text; text = ""; UpdateService.submitPassword(value) }
								}
								Text { visible: UpdateService.passwordError; text: I18n.tr("Incorrect password"); color: ThemeManager.error; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
								RowLayout {
									Layout.fillWidth: true
									Item { Layout.fillWidth: true }
									SettingBtn { label: I18n.tr("Cancel"); onClicked: { _sudoPassword.text = ""; UpdateService.cancelPassword() } }
									SettingBtn { label: UpdateService.authenticating ? I18n.tr("Checking…") : I18n.tr("Authorize"); enabled: !UpdateService.authenticating; onClicked: { const value = _sudoPassword.text; _sudoPassword.text = ""; UpdateService.submitPassword(value) } }
								}
							}
						}
					}

					// Desktop widgets ---------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "widgets"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 10
						SettingSection { text: I18n.tr("Desktop widgets") }
						Text { Layout.fillWidth: true; text: I18n.tr("Choose which widgets appear on this monitor, then enter edit mode to drag them into place."); wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
						SettingToggle { label: I18n.tr("Clock"); path: "desktopWidgets." + root.modelData.name + ".clock.visible"; def: true }
						SettingToggle { label: I18n.tr("Calendar"); path: "desktopWidgets." + root.modelData.name + ".calendar.visible"; def: true }
						SettingToggle { label: I18n.tr("System performance"); path: "desktopWidgets." + root.modelData.name + ".system.visible"; def: true }
						SettingToggle { label: I18n.tr("Media player"); path: "desktopWidgets." + root.modelData.name + ".media.visible"; def: true }
						SettingToggle { label: I18n.tr("Weather"); path: "desktopWidgets." + root.modelData.name + ".weather.visible"; def: false }
						SettingToggle { label: I18n.tr("Today's agenda"); path: "desktopWidgets." + root.modelData.name + ".agenda.visible"; def: false }
						SettingToggle { label: I18n.tr("Storage"); path: "desktopWidgets." + root.modelData.name + ".storage.visible"; def: false }
						SettingToggle { label: I18n.tr("Privacy"); path: "desktopWidgets." + root.modelData.name + ".privacy.visible"; def: false }
						RowLayout {
							Layout.fillWidth: true
							Layout.topMargin: 8
							Item { Layout.fillWidth: true }
							SettingBtn {
								label: DesktopWidgetService.editMode ? I18n.tr("Finish editing") : I18n.tr("Edit positions")
								onClicked: {
									DesktopWidgetService.toggleEdit()
									if (DesktopWidgetService.editMode) SettingsUi.hide()
								}
							}
						}
					}

					// LocalSend ----------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "localsend"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 10
						SettingSection { text: I18n.tr("Nearby sharing") }
						Text { Layout.fillWidth: true; text: I18n.tr("Nodalix stays available on your local network and receives files like AirDrop."); wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
						RowLayout {
							Layout.fillWidth: true; spacing: 8
							TextField {
								id: _localSendAlias
								Layout.fillWidth: true; implicitHeight: 34
								text: LocalSendService.alias
								placeholderText: I18n.tr("Visible device name")
								placeholderTextColor: ThemeManager.onSurfaceVariant
								color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
								background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh; border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
								onAccepted: LocalSendService.setAlias(text)
							}
							SettingBtn { label: I18n.tr("Save"); onClicked: LocalSendService.setAlias(_localSendAlias.text) }
						}
						SettingSection { text: I18n.tr("Favorite devices"); Layout.topMargin: 12 }
						Text { Layout.fillWidth: true; text: I18n.tr("Files from favorite devices are accepted automatically. Only favorite devices should be trusted."); wrapMode: Text.WordWrap; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
						Text { visible: LocalSendService.favorites.length === 0; text: I18n.tr("No favorite devices yet"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
						Repeater {
							model: LocalSendService.favorites
							delegate: Rectangle {
								required property var modelData
								Layout.fillWidth: true; implicitHeight: 48; radius: ThemeManager.chipRadius
								color: ThemeManager.surfaceContainerHigh
								RowLayout {
									anchors { fill: parent; margins: 10 }
									spacing: 10
									Text { text: "󰋑"; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 18 }
									Text { Layout.fillWidth: true; text: modelData.alias; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; elide: Text.ElideRight }
									SettingBtn { label: I18n.tr("Remove"); danger: true; onClicked: LocalSendService.setFavorite(modelData.fingerprint, false, modelData.alias) }
								}
							}
						}
					}

					// Connectivity -------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "connectivity"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Connectivity") }
						Text {
							Layout.fillWidth: true
							text: root._ethernetConnected
								? I18n.tr("Connected by Ethernet")
								: (root._connectedWifi
									? I18n.tr("Connected to") + " " + root._connectedWifi.name
									: I18n.tr("No active network connection"))
							color: (root._ethernetConnected || root._connectedWifi) ? ThemeManager.primary : ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						LiveToggle {
							label: I18n.tr("Wi-Fi")
							sub: root._connectedWifi ? root._connectedWifi.name : I18n.tr("Wireless networks")
							checked: Networking.wifiEnabled
							onToggled: value => Networking.wifiEnabled = value
						}
						LiveToggle {
							label: I18n.tr("Bluetooth")
							sub: root._bluetoothAdapter ? I18n.tr("Wireless devices") : I18n.tr("No Bluetooth adapter detected")
							checked: root._bluetoothAdapter?.enabled ?? false
							controlEnabled: root._bluetoothAdapter !== null
							onToggled: value => { if (root._bluetoothAdapter) root._bluetoothAdapter.enabled = value }
						}
						LiveToggle {
							label: "Tailscale"
							sub: SystemControlService.tailscaleActive && SystemControlService.tailscaleIp !== ""
								? SystemControlService.tailscaleIp : I18n.tr("Private VPN network")
							checked: SystemControlService.tailscaleActive
							controlEnabled: SystemControlService.tailscaleDetected && !SystemControlService.actionBusy
							onToggled: value => SystemControlService.toggleTailscale()
						}
						RowLayout {
							Layout.fillWidth: true; Layout.topMargin: 8
							Text {
								Layout.fillWidth: true
								text: SystemControlService.statusText
								color: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: 10
								elide: Text.ElideRight
							}
							SettingBtn {
								label: I18n.tr("Refresh")
								enabled: !SystemControlService.loading
								onClicked: SystemControlService.refresh()
							}
						}
					}

					// Sound --------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "sound"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Sound") }
						LiveSlider {
							label: I18n.tr("Output volume")
							sub: AudioService.sink?.description ?? I18n.tr("No output device")
							value: AudioService.sinkVolume * 100
							from: 0; to: 150; unit: "%"
							onAdjusted: value => AudioService.setSinkVolume(value / 100)
						}
						LiveToggle {
							label: I18n.tr("Mute output")
							checked: AudioService.sinkMuted
							controlEnabled: AudioService.sink !== null
							onToggled: value => AudioService.toggleSinkMute()
						}
						LiveSlider {
							label: I18n.tr("Microphone level")
							sub: AudioService.source?.description ?? I18n.tr("No input device")
							value: AudioService.sourceVolume * 100
							from: 0; to: 100; unit: "%"
							onAdjusted: value => AudioService.setSourceVolume(value / 100)
						}
						LiveToggle {
							label: I18n.tr("Mute microphone")
							checked: AudioService.sourceMuted
							controlEnabled: AudioService.source !== null
							onToggled: value => AudioService.toggleSourceMute()
						}

						SettingSection { text: I18n.tr("Output devices") }
						Text {
							visible: AudioService.sinks.length === 0
							text: I18n.tr("No output devices detected")
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: AudioService.sinks
							delegate: AudioNodeRow {
								required property var modelData
								node: modelData
								selected: AudioService.sink?.id === modelData.id
								onClicked: AudioService.setDefaultSink(modelData)
							}
						}
						SettingSection { text: I18n.tr("Input devices") }
						Text {
							visible: AudioService.sources.length === 0
							text: I18n.tr("No input devices detected")
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: AudioService.sources
							delegate: AudioNodeRow {
								required property var modelData
								node: modelData
								selected: AudioService.source?.id === modelData.id
								onClicked: AudioService.setDefaultSource(modelData)
							}
						}
					}

					// Power --------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "power"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Power profile") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Choose between lower consumption and maximum performance.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						RowLayout {
							Layout.fillWidth: true; spacing: 6
							Repeater {
								model: [
									{ value: PowerProfile.PowerSaver, label: I18n.tr("Power Saver"), icon: "󰾆", available: true },
									{ value: PowerProfile.Balanced, label: I18n.tr("Balanced"), icon: "󰾅", available: true },
									{ value: PowerProfile.Performance, label: I18n.tr("Performance"), icon: "󰓅", available: PowerProfiles.hasPerformanceProfile }
								]
								delegate: Rectangle {
									required property var modelData
									visible: modelData.available
									Layout.fillWidth: true; implicitHeight: 54; radius: ThemeManager.chipRadius + 2
									readonly property bool selected: PowerProfiles.profile === modelData.value
									color: selected ? ThemeManager.secondaryContainer : ThemeManager.surfaceContainerLow
									border.width: 1; border.color: selected ? ThemeManager.primary : ThemeManager.outlineVariant
									ColumnLayout {
										anchors.centerIn: parent; spacing: 1
										Text { Layout.alignment: Qt.AlignHCenter; text: modelData.icon; color: parent.parent.selected ? ThemeManager.primary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 18 }
										Text { Layout.alignment: Qt.AlignHCenter; text: modelData.label; color: parent.parent.selected ? ThemeManager.primary : ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
									}
									TapHandler { onTapped: PowerProfiles.profile = modelData.value }
								}
							}
						}
						Text {
							visible: PowerProfiles.degradationReason !== PerformanceDegradationReason.None
							Layout.fillWidth: true
							text: I18n.tr("Performance is temporarily limited by the system")
							color: ThemeManager.error
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}

						SettingSection { text: I18n.tr("Session") }
						RowLayout {
							Layout.fillWidth: true; spacing: 8
							SettingBtn { label: I18n.tr("Lock"); onClicked: { SettingsUi.hide(); LockService.lock() } }
							SettingBtn { label: I18n.tr("Suspend"); onClicked: Quickshell.execDetached(["systemctl", "suspend"]) }
							SettingBtn { label: I18n.tr("Hibernate"); onClicked: Quickshell.execDetached(["systemctl", "hibernate"]) }
							Item { Layout.fillWidth: true }
						}
						Text {
							Layout.fillWidth: true
							text: I18n.tr("Automatic locking is configured in Security.")
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
					}

					// Privacy ------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "privacy"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Privacy") }
						RowLayout {
							Layout.fillWidth: true; spacing: 8
							PrivacyState { icon: "󰍬"; label: I18n.tr("Microphone"); active: PrivacyService.microphoneActive }
							PrivacyState { icon: "󰄀"; label: I18n.tr("Camera"); active: PrivacyService.cameraActive }
							PrivacyState { icon: "󰍎"; label: I18n.tr("Location"); active: PrivacyService.locationActive }
						}
						LiveToggle {
							label: I18n.tr("Mute microphone")
							sub: I18n.tr("Prevents applications from hearing the system microphone")
							checked: AudioService.sourceMuted
							controlEnabled: AudioService.source !== null
							onToggled: value => AudioService.toggleSourceMute()
						}
						LiveToggle {
							label: I18n.tr("Do Not Disturb")
							sub: I18n.tr("Silence notification sounds and pop-ups")
							checked: NotificationService.doNotDisturb
							onToggled: value => {
								NotificationService.doNotDisturb = value
								SettingsService.set("notifications.dndDefault", value)
							}
						}
						Text {
							Layout.fillWidth: true; Layout.topMargin: 8
							text: I18n.tr("Privacy indicators around the clock light up only while a device or location service is in use.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
					}

					// Date and time ------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "date-time"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Date and time") }
						LiveToggle {
							label: I18n.tr("Automatic date and time")
							sub: SystemControlService.ntpSynchronized ? I18n.tr("Clock synchronized") : I18n.tr("Waiting for synchronization")
							checked: SystemControlService.ntpEnabled
							controlEnabled: !SystemControlService.actionBusy
							onToggled: value => SystemControlService.setNtp(value)
						}
						SettingRowBase {
							label: I18n.tr("Time zone")
							sub: I18n.tr("Use a region such as Europe/Madrid")
							TextField {
								id: _timezoneField
								Layout.preferredWidth: 190; implicitHeight: 32
								text: SystemControlService.timezone
								color: ThemeManager.onSurface
								placeholderTextColor: ThemeManager.onSurfaceVariant
								font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
								leftPadding: 10; rightPadding: 10
								background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh; border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
								onAccepted: SystemControlService.setTimezone(text)
							}
							SettingBtn {
								label: I18n.tr("Apply")
								enabled: !SystemControlService.actionBusy
								onClicked: SystemControlService.setTimezone(_timezoneField.text)
							}
						}
						Text {
							visible: SystemControlService.statusText !== ""
							Layout.fillWidth: true
							text: SystemControlService.statusText
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
					}

					// Default applications ----------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "default-apps"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 8

						SettingSection { text: I18n.tr("Default applications") }
						Text {
							Layout.fillWidth: true
							text: I18n.tr("These applications will be used by Nodalix and other compatible programs when opening links and files.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						DefaultAppPicker { label: I18n.tr("Web browser"); role: "browser"; currentId: SystemControlService.defaultBrowser }
						DefaultAppPicker { label: I18n.tr("File manager"); role: "files"; currentId: SystemControlService.defaultFileManager }
						DefaultAppPicker { label: I18n.tr("Email"); role: "mail"; currentId: SystemControlService.defaultMail }
						DefaultAppPicker { label: I18n.tr("Text files"); role: "text"; currentId: SystemControlService.defaultText }
						DefaultAppPicker { label: I18n.tr("Images"); role: "image"; currentId: SystemControlService.defaultImage }
						DefaultAppPicker { label: "PDF"; role: "pdf"; currentId: SystemControlService.defaultPdf }
						Text {
							visible: SystemControlService.statusText !== ""
							Layout.fillWidth: true
							text: SystemControlService.statusText
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: 10
						}
					}

					// About --------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "about"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 7

						SettingSection { text: I18n.tr("About this system") }
						SystemInfoRow { icon: "󰌢"; label: I18n.tr("Device name"); value: SystemControlService.hostName }
						SystemInfoRow { icon: "󰣇"; label: I18n.tr("Operating system"); value: SystemControlService.osName }
						SystemInfoRow { icon: "󰻠"; label: I18n.tr("Kernel"); value: SystemControlService.kernel }
						SystemInfoRow { icon: "󰍛"; label: "CPU"; value: SystemControlService.cpu }
						SystemInfoRow { icon: "󰘚"; label: I18n.tr("Memory"); value: SystemControlService.memory }
						SystemInfoRow { icon: "󰢮"; label: "GPU"; value: SystemControlService.gpu }
						RowLayout {
							Layout.fillWidth: true; Layout.topMargin: 8
							Item { Layout.fillWidth: true }
							SettingBtn { label: I18n.tr("Refresh"); enabled: !SystemControlService.loading; onClicked: SystemControlService.refresh() }
						}
					}

					// Dependencies --------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "dependencies"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 6
						SettingSection { text: I18n.tr("Optional dependencies") }
						Text {
							Layout.fillWidth: true; Layout.bottomMargin: 6
							text: I18n.tr("Optional features need these. Nothing is installed automatically.")
							wrapMode: Text.WordWrap
							color: ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
						}
						Repeater {
							model: Object.keys(DependencyService.deps)
							delegate: RowLayout {
								required property var modelData
								Layout.fillWidth: true
								spacing: 10
								readonly property bool ok: DependencyService.available(modelData)
								Text { text: ok ? "󰄬" : "󰅖"; color: ok ? "#7bd88f" : ThemeManager.error
										font.family: ThemeManager.fontFamily; font.pixelSize: 14 }
								ColumnLayout {
									Layout.fillWidth: true; spacing: 0
									Text { text: modelData + "  ·  " + DependencyService.desc(modelData)
											color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
									Text { visible: !parent.parent.ok; text: I18n.tr("install: ") + DependencyService.pkg(modelData)
											color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
								}
							}
						}
						Rectangle {
							Layout.topMargin: 8
							implicitWidth: _recheck.implicitWidth + 24; implicitHeight: 30
							radius: ThemeManager.chipRadius; color: _rcH.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow
							border.width: 1; border.color: ThemeManager.outlineVariant
							Text { id: _recheck; anchors.centerIn: parent; text: I18n.tr("Re-check"); color: ThemeManager.onSurface
									font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
							HoverHandler { id: _rcH }
							TapHandler { onTapped: DependencyService.recheck() }
						}
					}

					// Advanced ------------------------------------------------------
					ColumnLayout {
						visible: SettingsUi.category === "advanced"
						Layout.fillWidth: true
						Layout.margins: 20
						spacing: 10
						SettingSection { text: I18n.tr("Advanced") }
						Row {
							spacing: 10
							SettingBtn { label: I18n.tr("Open settings.json"); onClicked: _open.running = true }
							SettingBtn { label: I18n.tr("Reset to defaults"); danger: true; onClicked: SettingsService.reset() }
						}
						Process { id: _open; command: ["xdg-open", SettingsService._path]; running: false }
					}
				}
			}
		}
	}

	// ── Monitor confirm-or-revert dialog ──────────────────────────────────────
	// Applied display changes auto-revert after a countdown unless kept (a bad
	// mode can black out a screen). Rendered above the card.
	Rectangle {
		anchors.fill: parent
		visible: HyprlandConfigService.monitorConfirmPending
		color: Qt.rgba(0, 0, 0, 0.55)
		MouseArea { anchors.fill: parent }   // swallow clicks to the card

		Rectangle {
			anchors.centerIn: parent
			width: 360
			implicitHeight: _cdCol.implicitHeight + 40
			radius: ThemeManager.panelRadius + 4
			color: ThemeManager.surfaceContainer
			border.width: 1; border.color: ThemeManager.outlineVariant
			layer.enabled: true
			layer.effect: Elevation { level: 4 }

			ColumnLayout {
				id: _cdCol
				anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
				spacing: 12

				Text {
					Layout.fillWidth: true
					text: I18n.tr("Keep these display settings?")
					color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily
					font.pixelSize: ThemeManager.fontSizeLg; font.bold: true; wrapMode: Text.WordWrap
				}
				Text {
					Layout.fillWidth: true
					text: I18n.tr("Reverting to the previous settings in ") + HyprlandConfigService.monitorCountdown + " s…"
					color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily
					font.pixelSize: ThemeManager.fontSizeSm; wrapMode: Text.WordWrap
				}
				RowLayout {
					Layout.fillWidth: true
					spacing: 10
					Item { Layout.fillWidth: true }
					SettingBtn { label: I18n.tr("Revert"); danger: true; onClicked: HyprlandConfigService.revertMonitors() }
					SettingBtn { label: I18n.tr("Keep changes"); onClicked: HyprlandConfigService.confirmMonitors() }
				}
			}
		}
	}

	// ── Reusable controls ─────────────────────────────────────────────────────
	component SettingSection: Text {
		Layout.topMargin: 10
		Layout.bottomMargin: 2
		color: ThemeManager.primary
		font.family: ThemeManager.fontFamily
		font.pixelSize: ThemeManager.fontSizeSm
		font.bold: true
	}
	component SettingRowBase: RowLayout {
		id: rowBase
		property string label: ""
		property string sub: ""
		property string dep: ""
		Layout.fillWidth: true
		spacing: 10
		readonly property bool depOk: dep === "" || DependencyService.available(dep)
		ColumnLayout {
			Layout.fillWidth: true
			spacing: 0
			RowLayout {
				spacing: 6
				Text { text: rowBase.label; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeMd }
				Rectangle {
					visible: rowBase.dep !== "" && !rowBase.depOk
					implicitWidth: _dt.implicitWidth + 10; implicitHeight: 16; radius: 8
					color: Qt.rgba(ThemeManager.error.r, ThemeManager.error.g, ThemeManager.error.b, 0.18)
					Text { id: _dt; anchors.centerIn: parent; text: I18n.tr("needs ") + (rowBase.dep ? DependencyService.pkg(rowBase.dep) : "")
							color: ThemeManager.error; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
				}
			}
			Text { visible: rowBase.sub !== ""; text: rowBase.sub; color: ThemeManager.onSurfaceVariant
					font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
		}
	}
	component LiveToggle: SettingRowBase {
		id: liveToggle
		property bool checked: false
		property bool controlEnabled: true
		signal toggled(bool value)
		Rectangle {
			implicitWidth: 40; implicitHeight: 22; radius: 11
			opacity: liveToggle.controlEnabled ? 1 : 0.4
			color: liveToggle.checked ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
			Behavior on color { ColorAnimation { duration: 120 } }
			Rectangle {
				width: 16; height: 16; radius: 8; y: 3
				x: liveToggle.checked ? parent.width - width - 3 : 3
				color: liveToggle.checked ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
				Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
			}
			TapHandler {
				enabled: liveToggle.controlEnabled
				onTapped: liveToggle.toggled(!liveToggle.checked)
			}
		}
	}
	component LiveSlider: SettingRowBase {
		id: liveSlider
		property real value: 0
		property real from: 0
		property real to: 100
		property real step: 1
		property string unit: ""
		signal adjusted(real value)
		readonly property real shownValue: Math.max(from, Math.min(to, value))
		Text {
			text: Math.round(liveSlider.shownValue) + liveSlider.unit
			color: ThemeManager.onSurfaceVariant
			font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
			Layout.rightMargin: 8
		}
		Rectangle {
			id: liveTrack
			Layout.preferredWidth: 160; implicitHeight: 6; radius: 3
			color: ThemeManager.surfaceContainerHigh
			readonly property real fraction: (liveSlider.shownValue - liveSlider.from) / Math.max(1, liveSlider.to - liveSlider.from)
			Rectangle {
				anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
				width: liveTrack.width * liveTrack.fraction; radius: 3; color: ThemeManager.primary
			}
			Rectangle {
				width: 14; height: 14; radius: 7; y: -4
				x: Math.max(0, Math.min(liveTrack.width - width, liveTrack.width * liveTrack.fraction - width / 2))
				color: ThemeManager.primary
			}
			MouseArea {
				anchors.fill: parent; anchors.margins: -6
				onPressed: mouse => setFromX(mouse.x)
				onPositionChanged: mouse => { if (pressed) setFromX(mouse.x) }
				function setFromX(x) {
					const f = Math.max(0, Math.min(1, (x - 6) / liveTrack.width))
					const raw = liveSlider.from + f * (liveSlider.to - liveSlider.from)
					liveSlider.adjusted(Math.round(raw / liveSlider.step) * liveSlider.step)
				}
			}
		}
	}
	component AudioNodeRow: Rectangle {
		id: audioNode
		property var node: null
		property bool selected: false
		signal clicked()
		Layout.fillWidth: true; implicitHeight: 38; radius: ThemeManager.chipRadius
		color: selected ? ThemeManager.secondaryContainer : (_audioHover.hovered ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow)
		border.width: 1; border.color: selected ? ThemeManager.primary : ThemeManager.outlineVariant
		RowLayout {
			anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
			spacing: 8
			Text { text: audioNode.selected ? "󰄬" : "󰓃"; color: audioNode.selected ? ThemeManager.primary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 14 }
			Text {
				Layout.fillWidth: true
				text: audioNode.node?.description || audioNode.node?.name || I18n.tr("Unknown")
				color: audioNode.selected ? ThemeManager.primary : ThemeManager.onSurface
				font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
				elide: Text.ElideRight
			}
			Text { visible: audioNode.selected; text: I18n.tr("Default"); color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
		}
		HoverHandler { id: _audioHover; cursorShape: Qt.PointingHandCursor }
		TapHandler { onTapped: audioNode.clicked() }
	}
	component PrivacyState: Rectangle {
		id: privacyState
		property string icon: ""
		property string label: ""
		property bool active: false
		Layout.fillWidth: true; implicitHeight: 68; radius: ThemeManager.chipRadius + 2
		color: active
			? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.14)
			: ThemeManager.surfaceContainerLow
		border.width: 1; border.color: active ? ThemeManager.primary : ThemeManager.outlineVariant
		ColumnLayout {
			anchors.centerIn: parent; spacing: 2
			Text { Layout.alignment: Qt.AlignHCenter; text: privacyState.icon; color: privacyState.active ? ThemeManager.primary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 19 }
			Text { Layout.alignment: Qt.AlignHCenter; text: privacyState.label; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
			Text { Layout.alignment: Qt.AlignHCenter; text: I18n.tr(privacyState.active ? "In use" : "Not in use"); color: privacyState.active ? ThemeManager.primary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
		}
	}
	component DefaultAppPicker: SettingRowBase {
		id: appPicker
		property string role: ""
		property string currentId: ""
		readonly property var currentApp: AppService.byKey(currentId)
		IconImage {
			visible: appPicker.currentApp !== null
			implicitWidth: 24; implicitHeight: 24
			source: appPicker.currentApp ? AppService.iconFor(appPicker.currentApp) : ""
		}
		ComboBox {
			id: appCombo
			Layout.preferredWidth: 235; implicitHeight: 34
			model: AppService.apps
			textRole: "name"
			currentIndex: root._defaultAppIndex(appPicker.currentId)
			displayText: currentIndex >= 0 ? currentText : (appPicker.currentId || I18n.tr("Not set"))
			font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
			contentItem: Text {
				leftPadding: 10; rightPadding: 28
				text: appCombo.displayText
				color: ThemeManager.onSurface
				font: appCombo.font
				verticalAlignment: Text.AlignVCenter
				elide: Text.ElideRight
			}
			indicator: Text {
				x: appCombo.width - width - 10; anchors.verticalCenter: parent.verticalCenter
				text: "󰅀"; color: ThemeManager.onSurfaceVariant
				font.family: ThemeManager.fontFamily; font.pixelSize: 13
			}
			background: Rectangle {
				radius: ThemeManager.chipRadius
				color: ThemeManager.surfaceContainerHigh
				border.width: 1; border.color: appCombo.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant
			}
			onActivated: index => {
				const app = AppService.apps[index]
				if (app) SystemControlService.setDefault(appPicker.role, AppService.keyOf(app))
			}
		}
	}
	component SystemInfoRow: Rectangle {
		id: infoRow
		property string icon: ""
		property string label: ""
		property string value: ""
		Layout.fillWidth: true; implicitHeight: 48; radius: ThemeManager.chipRadius
		color: ThemeManager.surfaceContainerLow
		border.width: 1; border.color: ThemeManager.outlineVariant
		RowLayout {
			anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
			spacing: 10
			Text { text: infoRow.icon; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 17 }
			Text { text: infoRow.label; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
			Item { Layout.fillWidth: true }
			Text {
				Layout.maximumWidth: parent.width * 0.62
				text: infoRow.value
				color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
				font.weight: Font.Medium; elide: Text.ElideRight
			}
		}
	}
	component SettingToggle: SettingRowBase {
		id: tg
		property string path: ""
		property bool def: false
		property var applyFn: null
		readonly property bool on: SettingsService.get(path, def)
		Rectangle {
			implicitWidth: 40; implicitHeight: 22; radius: 11
			opacity: tg.depOk ? 1 : 0.4
			color: tg.on ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
			Behavior on color { ColorAnimation { duration: 120 } }
			Rectangle {
				width: 16; height: 16; radius: 8
				y: 3; x: tg.on ? parent.width - width - 3 : 3
				color: tg.on ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
				Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
			}
			TapHandler { enabled: tg.depOk; onTapped: { SettingsService.set(tg.path, !tg.on); if (tg.applyFn) tg.applyFn() } }
		}
	}
	component SettingSlider: SettingRowBase {
		id: sl
		property string path: ""
		property real def: 0
		property real from: 0
		property real to: 100
		property string unit: ""
		property var applyFn: null
		readonly property real val: SettingsService.get(path, def)
		Text { text: Math.round(sl.val) + sl.unit; color: ThemeManager.onSurfaceVariant
				font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm; Layout.rightMargin: 8 }
		Rectangle {
			id: track
			Layout.preferredWidth: 160; implicitHeight: 6; radius: 3
			color: ThemeManager.surfaceContainerHigh
			readonly property real _frac: Math.max(0, Math.min(1, (sl.val - sl.from) / (sl.to - sl.from)))
			Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
						width: track.width * track._frac; radius: 3; color: ThemeManager.primary }
			Rectangle { width: 14; height: 14; radius: 7; color: ThemeManager.primary
						y: -4; x: Math.max(0, Math.min(track.width - width, track.width * track._frac - width / 2)) }
			MouseArea {
				anchors.fill: parent; anchors.margins: -6
				onPressed: (e) => _set(e.x); onPositionChanged: (e) => { if (pressed) _set(e.x) }
				onReleased: if (sl.applyFn) sl.applyFn()
				function _set(x) {
					const f = Math.max(0, Math.min(1, (x - 6) / track.width))
					SettingsService.set(sl.path, Math.round(sl.from + f * (sl.to - sl.from)))
				}
			}
		}
	}
	component HdrSlider: SettingRowBase {
		id: hs
		property string monitorName: ""
		property string keyName: ""
		property real def: 1.0
		property real from: 0.0
		property real to: 2.0
		property real step: 0.05
		property int decimals: 2
		property string unit: ""
		readonly property real val: SettingsService.get("hypr.monitors." + monitorName + "." + keyName, def)
		Text {
			text: Number(hs.val).toFixed(hs.decimals) + hs.unit
			color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily
			font.pixelSize: ThemeManager.fontSizeSm; Layout.rightMargin: 8
		}
		Rectangle {
			id: hdrTrack
			Layout.preferredWidth: 160; implicitHeight: 6; radius: 3
			color: ThemeManager.surfaceContainerLow
			readonly property real frac: Math.max(0, Math.min(1, (hs.val - hs.from) / (hs.to - hs.from)))
			Rectangle {
				anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
				width: hdrTrack.width * hdrTrack.frac; radius: 3; color: ThemeManager.primary
			}
			Rectangle {
				width: 14; height: 14; radius: 7; y: -4
				x: Math.max(0, Math.min(hdrTrack.width - width, hdrTrack.width * hdrTrack.frac - width / 2))
				color: ThemeManager.primary
			}
			MouseArea {
				anchors.fill: parent; anchors.margins: -6
				onPressed: (e) => setValue(e.x)
				onPositionChanged: (e) => { if (pressed) setValue(e.x) }
				function setValue(x) {
					const f = Math.max(0, Math.min(1, (x - 6) / hdrTrack.width))
					const raw = hs.from + f * (hs.to - hs.from)
					const value = Math.round(raw / hs.step) * hs.step
					HyprlandConfigService.stageMonitor(hs.monitorName, hs.keyName, Number(value.toFixed(hs.decimals)))
				}
			}
		}
	}
	component SettingSeg: SettingRowBase {
		id: seg
		property string path: ""
		property string def: ""
		property var options: []
		property var keys: []
		property var applyFn: null
		property bool enabled: true
		readonly property string cur: SettingsService.get(path, def)
		Row {
			spacing: 0
			Repeater {
				model: seg.options
				delegate: Rectangle {
					required property var modelData
					required property int index
					implicitWidth: _st.implicitWidth + 22; implicitHeight: 28
					readonly property bool sel: seg.cur === seg.keys[index]
					opacity: seg.enabled ? 1 : 0.55
					color: sel ? ThemeManager.primary : ThemeManager.surfaceContainerHigh
					border.width: 1; border.color: ThemeManager.outlineVariant
					Text { id: _st; anchors.centerIn: parent; text: modelData
							color: sel ? ThemeManager.onPrimary : ThemeManager.onSurfaceVariant
							font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
					TapHandler { enabled: seg.enabled; onTapped: { SettingsService.set(seg.path, seg.keys[index]); if (seg.applyFn) seg.applyFn(seg.keys[index]) } }
				}
			}
		}
	}
	component SettingColor: RowLayout {
		id: clr
		property string role: ""
		Layout.fillWidth: true
		spacing: 10
		Rectangle {
			width: 22; height: 22; radius: 5
			color: ThemeManager[clr.role] !== undefined ? ThemeManager[clr.role] : "transparent"
			border.width: 1; border.color: ThemeManager.outlineVariant
		}
		Text {
			Layout.fillWidth: true; text: clr.role
			color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
		}
		TextField {
			Layout.preferredWidth: 100; implicitHeight: 26
			text: ThemeManager.roleHex(clr.role)
			color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm
			leftPadding: 8; rightPadding: 8
			background: Rectangle { radius: ThemeManager.chipRadius; color: ThemeManager.surfaceContainerHigh
									border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant }
			onEditingFinished: if (text !== ThemeManager.roleHex(clr.role)) ThemeManager.setRole(clr.role, text)
		}
	}
	component SettingText: SettingRowBase {
		id: tx
		property string path: ""
		property string def: ""
		property string placeholder: ""
		property var applyFn: null
		TextField {
			Layout.preferredWidth: 170
			implicitHeight: 30
			text: SettingsService.get(tx.path, tx.def)
			placeholderText: tx.placeholder
			placeholderTextColor: ThemeManager.onSurfaceVariant
			color: ThemeManager.onSurface
			font.family: ThemeManager.fontFamily
			font.pixelSize: ThemeManager.fontSizeSm
			leftPadding: 10; rightPadding: 10
			background: Rectangle {
				radius: ThemeManager.chipRadius
				color: ThemeManager.surfaceContainerHigh
				border.width: 1; border.color: parent.activeFocus ? ThemeManager.primary : ThemeManager.outlineVariant
			}
			onEditingFinished: { SettingsService.set(tx.path, text); if (tx.applyFn) tx.applyFn() }
		}
	}
	component SettingBtn: Rectangle {
		id: btn
		property string label: ""
		property bool danger: false
		property bool enabled: true
		signal clicked()
		implicitWidth: _bt.implicitWidth + 26; implicitHeight: 32
		radius: ThemeManager.chipRadius
		opacity: enabled ? 1 : 0.4
		color: (enabled && _bH.hovered) ? ThemeManager.surfaceContainerHigh : ThemeManager.surfaceContainerLow
		border.width: 1; border.color: danger ? ThemeManager.error : ThemeManager.outlineVariant
		Text { id: _bt; anchors.centerIn: parent; text: btn.label
				color: btn.danger ? ThemeManager.error : ThemeManager.onSurface
				font.family: ThemeManager.fontFamily; font.pixelSize: ThemeManager.fontSizeSm }
		HoverHandler { id: _bH; enabled: btn.enabled; cursorShape: Qt.PointingHandCursor }
		TapHandler { enabled: btn.enabled; onTapped: btn.clicked() }
	}

	// Small round overlay button on a wallpaper tile (favorite / rotation).
	component WpTileBtn: Rectangle {
		id: wtb
		property string icon: ""
		property bool   active: false
		signal clicked()
		implicitWidth: 24; implicitHeight: 24; radius: 12
		color: active ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.9)
						: Qt.rgba(0, 0, 0, 0.45)
		Text {
			anchors.centerIn: parent
			text: wtb.icon
			color: wtb.active ? ThemeManager.onPrimary : "white"
			font.family: ThemeManager.fontFamily; font.pixelSize: 13
		}
		MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: wtb.clicked() }
	}
}
