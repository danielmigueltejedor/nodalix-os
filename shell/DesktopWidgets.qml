import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import "./theme"
import "./services"

PanelWindow {
    id: root
    required property var modelData
    screen: modelData
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Bottom
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

    property var now: new Date()
    property real diskTotal: 0
    property real diskUsed: 0
    readonly property string _todayKey: Qt.formatDate(now, "yyyy-MM-dd")
    readonly property var _todayEvents: CalendarService.eventsOn(_todayKey)
    Timer {
        interval: 30000
        running: SettingsService.get(root._prefix + "clock.visible", true)
              || SettingsService.get(root._prefix + "calendar.visible", true)
        repeat: true
        onTriggered: root.now = new Date()
    }
    readonly property string _prefix: "desktopWidgets." + modelData.name + "."
    function _formatBytes(value) {
        let size = Number(value) || 0
        const units = ["B", "KB", "MB", "GB", "TB"]
        let unit = 0
        while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
        return (unit < 2 ? Math.round(size) : size.toFixed(1)) + " " + units[unit]
    }

    Process {
        id: _diskProbe
        command: ["sh", "-c", "df -B1 --output=size,used / | tail -1"]
        stdout: StdioCollector {
            onStreamFinished: {
                const fields = text.trim().split(/\s+/)
                root.diskTotal = Number(fields[0]) || 0
                root.diskUsed = Number(fields[1]) || 0
            }
        }
    }
    Timer {
        interval: 300000; repeat: true
        running: SettingsService.get(root._prefix + "storage.visible", false)
        triggeredOnStart: true
        onTriggered: if (!_diskProbe.running) _diskProbe.running = true
    }
    Timer {
        interval: 900000; repeat: true
        running: SettingsService.get(root._prefix + "agenda.visible", false)
        triggeredOnStart: true
        onTriggered: CalendarService.loadMonth(root.now)
    }

    mask: Region {
        Region { x: 0; y: 0; width: DesktopWidgetService.editMode ? root.width : 0; height: DesktopWidgetService.editMode ? root.height : 0 }
        Region { x: mediaCard.x; y: mediaCard.y; width: !DesktopWidgetService.editMode && mediaCard.visible ? mediaCard.width : 0; height: !DesktopWidgetService.editMode && mediaCard.visible ? mediaCard.height : 0 }
    }

    Rectangle {
        id: _editor
        visible: DesktopWidgetService.editMode
        anchors { top: parent.top; horizontalCenter: parent.horizontalCenter; topMargin: ThemeManager.barHeight + 18 }
        z: 20
        width: Math.min(940, root.width - 36)
        height: _editorColumn.implicitHeight + 20
        radius: 24
        color: Qt.rgba(ThemeManager.surfaceContainerHigh.r, ThemeManager.surfaceContainerHigh.g, ThemeManager.surfaceContainerHigh.b, 0.96)
        border.width: 1; border.color: Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.28)
        ColumnLayout {
            id: _editorColumn
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                Text { text: "󰜬"; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 18 }
                Text { Layout.fillWidth: true; text: I18n.tr("Desktop widgets"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.weight: Font.Bold; font.pixelSize: 14 }
                Text { text: I18n.tr("Drag widgets to arrange them"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
                Rectangle {
                    width: 30; height: 30; radius: 15; color: _closeHover.hovered ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.16) : "transparent"
                    Text { anchors.centerIn: parent; text: "󰅖"; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 17 }
                    HoverHandler { id: _closeHover }
                    TapHandler { onTapped: DesktopWidgetService.closeEdit() }
                }
            }
            Flow {
                Layout.fillWidth: true
                spacing: 7
                WidgetToggle { label: I18n.tr("Clock"); setting: "clock"; defaultVisible: true }
                WidgetToggle { label: I18n.tr("Calendar"); setting: "calendar"; defaultVisible: true }
                WidgetToggle { label: I18n.tr("System performance"); setting: "system"; defaultVisible: true }
                WidgetToggle { label: I18n.tr("Media player"); setting: "media"; defaultVisible: true }
                WidgetToggle { label: I18n.tr("Weather"); setting: "weather" }
                WidgetToggle { label: I18n.tr("Today's agenda"); setting: "agenda" }
                WidgetToggle { label: I18n.tr("Storage"); setting: "storage" }
                WidgetToggle { label: I18n.tr("Privacy"); setting: "privacy" }
            }
        }
    }

    DesktopCard {
        widgetKey: "clock"; defaultX: 72; defaultY: 122; cardWidth: 300; cardHeight: 150
        icon: "󰥔"; title: I18n.tr("Clock"); accent: ThemeManager.primary
        ColumnLayout {
            anchors { fill: parent; leftMargin: 22; rightMargin: 22; topMargin: 18; bottomMargin: 16 }
            spacing: 0
            RowLayout {
                Layout.fillWidth: true
                Text { text: I18n.tr("Now").toUpperCase(); color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 10; font.weight: Font.DemiBold; font.letterSpacing: 1.4 }
                Item { Layout.fillWidth: true }
                Text { text: root.now.toLocaleDateString(Qt.locale(I18n.localeName), "yyyy"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10; opacity: 0.72 }
            }
            Text {
                Layout.fillWidth: true
                text: root.now.toLocaleTimeString(Qt.locale(I18n.localeName), SettingsService.get("bar.clock.seconds", false) ? "HH:mm:ss" : "HH:mm")
                color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 50; font.weight: Font.Bold; font.letterSpacing: -1.5
            }
            Text {
                Layout.fillWidth: true
                text: root.now.toLocaleDateString(Qt.locale(I18n.localeName), I18n.language === "es" ? "dddd, d 'de' MMMM" : "dddd, MMMM d")
                color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 13; font.capitalization: Font.Capitalize
            }
        }
    }
    DesktopCard {
        id: calendarCard
        widgetKey: "calendar"; defaultX: 72; defaultY: 294; cardWidth: 300; cardHeight: 278
        icon: "󰃭"; title: I18n.tr("Calendar"); accent: ThemeManager.tertiary
        readonly property var _first: new Date(root.now.getFullYear(), root.now.getMonth(), 1)
        readonly property int _offset: (_first.getDay() + 6) % 7
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 10
            RowLayout {
                Layout.fillWidth: true
                Text { text: root.now.toLocaleDateString(Qt.locale(I18n.localeName), "MMMM"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 20; font.weight: Font.Bold; font.capitalization: Font.Capitalize }
                Item { Layout.fillWidth: true }
                Rectangle {
                    implicitWidth: _year.implicitWidth + 16; implicitHeight: 25; radius: 13
                    color: Qt.rgba(calendarCard.accent.r, calendarCard.accent.g, calendarCard.accent.b, 0.13)
                    Text { id: _year; anchors.centerIn: parent; text: root.now.getFullYear(); color: calendarCard.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 11; font.weight: Font.DemiBold }
                }
            }
            GridLayout {
                Layout.fillWidth: true; Layout.fillHeight: true
                columns: 7; columnSpacing: 3; rowSpacing: 3
                Repeater {
                    model: 7
                    delegate: Text {
                        required property int index
                        Layout.fillWidth: true; Layout.preferredHeight: 20
                        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
                        text: new Date(2024, 0, 1 + index).toLocaleDateString(Qt.locale(I18n.localeName), "ddd").slice(0, 1).toUpperCase()
                        color: ThemeManager.onSurfaceVariant; opacity: 0.7
                        font.family: ThemeManager.fontFamily; font.pixelSize: 10; font.weight: Font.DemiBold
                    }
                }
                Repeater {
                    model: 42
                    delegate: Rectangle {
                        required property int index
                        readonly property var dayValue: new Date(root.now.getFullYear(), root.now.getMonth(), index - calendarCard._offset + 1)
                        readonly property bool inMonth: dayValue.getMonth() === root.now.getMonth()
                        readonly property bool today: inMonth && dayValue.getDate() === root.now.getDate()
                        Layout.fillWidth: true; Layout.fillHeight: true
                        radius: 9
                        color: today ? calendarCard.accent : "transparent"
                        Text {
                            anchors.centerIn: parent; text: parent.dayValue.getDate()
                            color: parent.today ? ThemeManager.onPrimary : (parent.inMonth ? ThemeManager.onSurface : ThemeManager.onSurfaceVariant)
                            opacity: parent.inMonth ? 1 : 0.32
                            font.family: ThemeManager.fontFamily; font.pixelSize: 11; font.weight: parent.today ? Font.Bold : Font.Normal
                        }
                    }
                }
            }
        }
    }
    DesktopCard {
        widgetKey: "system"; defaultX: 398; defaultY: 122; cardWidth: 350; cardHeight: 178
        icon: "󰍛"; title: I18n.tr("System performance"); accent: ThemeManager.secondary
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 12
            WidgetHeader { icon: "󰍛"; title: I18n.tr("System performance"); accent: ThemeManager.secondary }
            RowLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 8
                MetricTile { Layout.fillWidth: true; icon: "󰍛"; label: "CPU"; value: SystemMetricsService.cpu; display: Math.round(SystemMetricsService.cpu) + "%"; accent: ThemeManager.primary }
                MetricTile { Layout.fillWidth: true; icon: "󰘚"; label: "RAM"; value: SystemMetricsService.ram; display: Math.round(SystemMetricsService.ram) + "%"; accent: ThemeManager.secondary }
                MetricTile { Layout.fillWidth: true; icon: "󰔏"; label: I18n.tr("Temperature"); value: SystemMetricsService.temp >= 0 ? SystemMetricsService.temp : 0; display: SystemMetricsService.temp >= 0 ? Math.round(SystemMetricsService.temp) + "°" : "—"; accent: ThemeManager.tertiary }
            }
        }
    }
    DesktopCard {
        id: mediaCard
        widgetKey: "media"; defaultX: 398; defaultY: 316; cardWidth: 420; cardHeight: 178
        icon: "󰝚"; title: I18n.tr("Media player"); accent: ThemeManager.primary
        RowLayout {
            anchors { fill: parent; margins: 15 }
            spacing: 16
            ClippingRectangle {
                Layout.preferredWidth: 148; Layout.preferredHeight: 148; radius: 28
                color: ThemeManager.surfaceContainerHigh
                Image { anchors.fill: parent; source: MprisService.artUrl; fillMode: Image.PreserveAspectCrop; visible: MprisService.artUrl !== "" }
                Rectangle {
                    anchors.fill: parent; visible: MprisService.artUrl === ""
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.rgba(mediaCard.accent.r, mediaCard.accent.g, mediaCard.accent.b, 0.22) }
                        GradientStop { position: 1; color: Qt.rgba(ThemeManager.surfaceContainerHigh.r, ThemeManager.surfaceContainerHigh.g, ThemeManager.surfaceContainerHigh.b, 0.96) }
                    }
                }
                Text { anchors.centerIn: parent; visible: MprisService.artUrl === ""; text: "󰝚"; color: ThemeManager.primary; font.family: ThemeManager.fontFamily; font.pixelSize: 42 }
                Rectangle { anchors.fill: parent; radius: 28; color: "transparent"; border.width: 1; border.color: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.13) }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 4
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: I18n.tr("Now playing").toUpperCase(); color: mediaCard.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1.1 }
                    Item { Layout.fillWidth: true }
                    Rectangle { width: 7; height: 7; radius: 4; color: MprisService.playing ? mediaCard.accent : ThemeManager.onSurfaceVariant; opacity: MprisService.hasPlayer ? 1 : 0.35 }
                }
                Item { Layout.fillHeight: true }
                Text { Layout.fillWidth: true; text: MprisService.hasPlayer ? MprisService.title : I18n.tr("No media"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 17; font.weight: Font.Bold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: MprisService.hasPlayer ? MprisService.artist : I18n.tr("Play something to see it here"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 11; elide: Text.ElideRight }
                Item { Layout.preferredHeight: 3 }
                Rectangle {
                    Layout.fillWidth: true; implicitHeight: 5; radius: 3; color: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.10)
                    Rectangle { width: parent.width * MprisService.progress; height: parent.height; radius: 3; color: ThemeManager.primary; Behavior on width { NumberAnimation { duration: 180 } } }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: MprisService.fmt(MprisService.position); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
                    Item { Layout.fillWidth: true }
                    Text { text: MprisService.fmt(MprisService.length); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    MediaButton { icon: "󰒮"; onClicked: MprisService.previous() }
                    MediaButton { icon: MprisService.playing ? "󰏤" : "󰐊"; primary: true; onClicked: MprisService.playPause() }
                    MediaButton { icon: "󰒭"; onClicked: MprisService.next() }
                    Item { Layout.fillWidth: true }
                    Text { text: "󰎈"; color: ThemeManager.onSurfaceVariant; opacity: 0.65; font.family: ThemeManager.fontFamily; font.pixelSize: 15 }
                }
            }
        }
    }

    DesktopCard {
        id: weatherCard
        widgetKey: "weather"; defaultVisible: false
        defaultX: 826; defaultY: 122; cardWidth: 360; cardHeight: 190
        icon: "󰖐"; title: I18n.tr("Weather"); accent: ThemeManager.tertiary
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 10
            WidgetHeader { icon: WeatherService.icon; title: WeatherService.location !== "" ? WeatherService.location : I18n.tr("Weather"); accent: weatherCard.accent }
            RowLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 14
                ColumnLayout {
                    Layout.preferredWidth: 112; spacing: 0
                    Text { text: WeatherService.ok ? WeatherService.temp + WeatherService.unit : "—"; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 42; font.weight: Font.Bold }
                    Text { Layout.fillWidth: true; text: WeatherService.ok ? WeatherService.desc : I18n.tr("Weather unavailable"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 11; elide: Text.ElideRight }
                    Text { text: WeatherService.ok ? I18n.tr("Humidity") + "  " + WeatherService.humidity + "%" : ""; color: weatherCard.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
                }
                Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.08) }
                RowLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true; spacing: 5
                    Repeater {
                        model: WeatherService.forecast
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 3
                            Text { Layout.alignment: Qt.AlignHCenter; text: modelData.day; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10; font.weight: Font.DemiBold }
                            Text { Layout.alignment: Qt.AlignHCenter; text: modelData.icon; color: weatherCard.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 23 }
                            Text { Layout.alignment: Qt.AlignHCenter; text: WeatherService.conv(modelData.max) + "°  " + WeatherService.conv(modelData.min) + "°"; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
                        }
                    }
                }
            }
        }
    }

    DesktopCard {
        id: agendaCard
        widgetKey: "agenda"; defaultVisible: false
        defaultX: 826; defaultY: 336; cardWidth: 380; cardHeight: 222
        icon: "󰃭"; title: I18n.tr("Today's agenda"); accent: ThemeManager.primary
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 8
            WidgetHeader {
                icon: "󰃭"; title: I18n.tr("Today's agenda"); accent: agendaCard.accent
                trailing: root._todayEvents.length > 0 ? root._todayEvents.length + " " + I18n.tr(root._todayEvents.length === 1 ? "event" : "events") : ""
            }
            Item { Layout.fillWidth: true; Layout.fillHeight: true; visible: root._todayEvents.length === 0
                Column { anchors.centerIn: parent; spacing: 6
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: "󰃶"; color: agendaCard.accent; opacity: 0.65; font.family: ThemeManager.fontFamily; font.pixelSize: 28 }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: I18n.tr("No events today"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 12 }
                }
            }
            Repeater {
                model: root._todayEvents.slice(0, 3)
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true; spacing: 9
                    Rectangle { Layout.preferredWidth: 3; Layout.preferredHeight: 30; radius: 2; color: agendaCard.accent }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 0
                        Text { Layout.fillWidth: true; text: modelData.title; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 11; font.weight: Font.DemiBold; elide: Text.ElideRight }
                        Text { Layout.fillWidth: true; text: modelData.stime !== "" ? modelData.stime + (modelData.etime !== "" ? " – " + modelData.etime : "") : I18n.tr("All day"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9; elide: Text.ElideRight }
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true; implicitHeight: 28; radius: 14
                visible: CalendarService.reminders.length > 0
                color: Qt.rgba(agendaCard.accent.r, agendaCard.accent.g, agendaCard.accent.b, 0.11)
                RowLayout { anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                    Text { text: "󰂚"; color: agendaCard.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 13 }
                    Text { Layout.fillWidth: true; text: CalendarService.reminders.length + " " + I18n.tr(CalendarService.reminders.length === 1 ? "reminder" : "reminders"); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 10 }
                }
            }
        }
    }

    DesktopCard {
        id: storageCard
        widgetKey: "storage"; defaultVisible: false
        defaultX: 1232; defaultY: 122; cardWidth: 320; cardHeight: 170
        icon: "󰋊"; title: I18n.tr("Storage"); accent: ThemeManager.secondary
        readonly property real fraction: root.diskTotal > 0 ? Math.max(0, Math.min(1, root.diskUsed / root.diskTotal)) : 0
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 12
            WidgetHeader { icon: "󰋊"; title: I18n.tr("Storage"); accent: storageCard.accent; trailing: Math.round(storageCard.fraction * 100) + "%" }
            Rectangle {
                Layout.fillWidth: true; implicitHeight: 10; radius: 5
                color: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.09)
                Rectangle { width: parent.width * storageCard.fraction; height: parent.height; radius: 5; color: storageCard.accent; Behavior on width { NumberAnimation { duration: 320; easing.type: Easing.OutCubic } } }
            }
            RowLayout {
                Layout.fillWidth: true
                ColumnLayout { spacing: 1
                    Text { text: I18n.tr("In use"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
                    Text { text: root._formatBytes(root.diskUsed); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 15; font.weight: Font.Bold }
                }
                Item { Layout.fillWidth: true }
                ColumnLayout { spacing: 1
                    Text { Layout.alignment: Qt.AlignRight; text: I18n.tr("Free"); color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 9 }
                    Text { Layout.alignment: Qt.AlignRight; text: root._formatBytes(Math.max(0, root.diskTotal - root.diskUsed)); color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 15; font.weight: Font.Bold }
                }
            }
        }
    }

    DesktopCard {
        id: privacyCard
        widgetKey: "privacy"; defaultVisible: false
        defaultX: 1232; defaultY: 316; cardWidth: 340; cardHeight: 150
        icon: "󰒃"; title: I18n.tr("Privacy"); accent: PrivacyService.active ? ThemeManager.tertiary : ThemeManager.primary
        ColumnLayout {
            anchors { fill: parent; margins: 18; topMargin: 16 }
            spacing: 13
            WidgetHeader { icon: "󰒃"; title: I18n.tr("Privacy"); accent: privacyCard.accent; trailing: PrivacyService.active ? I18n.tr("In use") : I18n.tr("Protected") }
            RowLayout {
                Layout.fillWidth: true; spacing: 7
                PrivacyPill { Layout.fillWidth: true; icon: "󰍬"; label: I18n.tr("Microphone"); active: PrivacyService.microphoneActive }
                PrivacyPill { Layout.fillWidth: true; icon: "󰄀"; label: I18n.tr("Camera"); active: PrivacyService.cameraActive }
                PrivacyPill { Layout.fillWidth: true; icon: "󰍎"; label: I18n.tr("Location"); active: PrivacyService.locationActive }
            }
        }
    }

    component WidgetHeader: RowLayout {
        required property string icon
        required property string title
        property color accent: ThemeManager.primary
        property string trailing: ""
        spacing: 8
        Rectangle {
            Layout.preferredWidth: 28; Layout.preferredHeight: 28; radius: 10
            color: Qt.rgba(parent.accent.r, parent.accent.g, parent.accent.b, 0.14)
            Text { anchors.centerIn: parent; text: parent.parent.icon; color: parent.parent.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 15 }
        }
        Text { Layout.fillWidth: true; text: parent.title; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 13; font.weight: Font.Bold; elide: Text.ElideRight }
        Rectangle {
            visible: parent.trailing !== ""
            implicitWidth: _headerTrailing.implicitWidth + 14; implicitHeight: 24; radius: 12
            color: Qt.rgba(parent.accent.r, parent.accent.g, parent.accent.b, 0.11)
            Text { id: _headerTrailing; anchors.centerIn: parent; text: parent.parent.trailing; color: parent.parent.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 9; font.weight: Font.DemiBold }
        }
    }

    component MetricTile: Rectangle {
        id: metricTile
        required property string icon
        required property string label
        required property real value
        required property string display
        property color accent: ThemeManager.primary
        implicitHeight: 92; radius: 18
        color: Qt.rgba(accent.r, accent.g, accent.b, 0.08)
        border.width: 1; border.color: Qt.rgba(accent.r, accent.g, accent.b, 0.13)
        ColumnLayout {
            anchors { fill: parent; margins: 10 }
            spacing: 2
            RowLayout { Layout.fillWidth: true
                Text { text: metricTile.icon; color: metricTile.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 14 }
                Item { Layout.fillWidth: true }
                Text { text: metricTile.display; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 14; font.weight: Font.Bold }
            }
            Text { Layout.fillWidth: true; text: metricTile.label; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 8; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Item { Layout.fillHeight: true }
            Rectangle {
                Layout.fillWidth: true; implicitHeight: 4; radius: 2
                color: Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.09)
                Rectangle {
                    width: parent.width * Math.max(0, Math.min(100, metricTile.value)) / 100
                    height: parent.height; radius: 2; color: metricTile.accent
                    Behavior on width { NumberAnimation { duration: 280; easing.type: Easing.OutCubic } }
                }
            }
        }
    }

    component MediaButton: Rectangle {
        id: mediaButton
        required property string icon
        property bool primary: false
        signal clicked()
        implicitWidth: primary ? 38 : 32; implicitHeight: primary ? 38 : 32
        radius: width / 2
        color: primary ? ThemeManager.primary : (_mediaHover.hovered ? Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.10) : "transparent")
        scale: _mediaTap.pressed ? 0.90 : 1
        Behavior on scale { NumberAnimation { duration: 90 } }
        Text { anchors.centerIn: parent; text: mediaButton.icon; color: mediaButton.primary ? ThemeManager.onPrimary : ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: mediaButton.primary ? 19 : 16 }
        HoverHandler { id: _mediaHover }
        TapHandler { id: _mediaTap; onTapped: mediaButton.clicked() }
    }

    component PrivacyPill: Rectangle {
        id: privacyPill
        required property string icon
        required property string label
        required property bool active
        implicitHeight: 60; radius: 16
        color: active
            ? Qt.rgba(ThemeManager.tertiary.r, ThemeManager.tertiary.g, ThemeManager.tertiary.b, 0.17)
            : Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.055)
        border.width: 1
        border.color: active ? Qt.rgba(ThemeManager.tertiary.r, ThemeManager.tertiary.g, ThemeManager.tertiary.b, 0.35) : "transparent"
        Column { anchors.centerIn: parent; spacing: 3
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: privacyPill.icon; color: privacyPill.active ? ThemeManager.tertiary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 17 }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: privacyPill.label; color: ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 8; elide: Text.ElideRight }
        }
    }

    component DesktopCard: Rectangle {
        id: card
        property string widgetKey
        property string icon: "󰜬"
        property string title: widgetKey
        property color accent: ThemeManager.primary
        property bool defaultVisible: true
        property real defaultX: 0
        property real defaultY: 0
        property real cardWidth: 200
        property real cardHeight: 120
        default property alias content: contentItem.data
        visible: SettingsService.get(root._prefix + widgetKey + ".visible", defaultVisible)
        x: Math.max(0, Math.min(root.width - width, SettingsService.get(root._prefix + widgetKey + ".x", defaultX)))
        y: Math.max(ThemeManager.barHeight, Math.min(root.height - height, SettingsService.get(root._prefix + widgetKey + ".y", defaultY)))
        width: cardWidth; height: cardHeight
        radius: Math.max(20, ThemeManager.panelRadius + 6)
        color: Qt.rgba(ThemeManager.surfaceContainer.r, ThemeManager.surfaceContainer.g, ThemeManager.surfaceContainer.b, 0.90)
        border.width: DesktopWidgetService.editMode ? 2 : 1
        border.color: DesktopWidgetService.editMode ? card.accent : Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.11)
        clip: false
        Behavior on border.color { ColorAnimation { duration: 150 } }
        Rectangle {
            anchors { fill: parent; margins: 1 }
            radius: card.radius - 1
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: "transparent" }
                GradientStop { position: 1; color: Qt.rgba(card.accent.r, card.accent.g, card.accent.b, 0.055) }
            }
        }
        Item { id: contentItem; anchors.fill: parent }
        DragHandler {
            enabled: DesktopWidgetService.editMode
            target: card
            xAxis.minimum: 0; xAxis.maximum: root.width - card.width
            yAxis.minimum: ThemeManager.barHeight; yAxis.maximum: root.height - card.height
            onActiveChanged: if (!active) {
                SettingsService.set(root._prefix + card.widgetKey + ".x", Math.round(card.x))
                SettingsService.set(root._prefix + card.widgetKey + ".y", Math.round(card.y))
            }
        }
        Text {
            visible: DesktopWidgetService.editMode
            anchors { right: parent.right; top: parent.top; margins: 7 }
            width: 28; height: 28
            text: "󰆾"; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
            color: ThemeManager.error; font.family: ThemeManager.fontFamily; font.pixelSize: 14
            Rectangle { anchors.fill: parent; z: -1; radius: 14; color: Qt.rgba(ThemeManager.error.r, ThemeManager.error.g, ThemeManager.error.b, 0.13) }
            TapHandler { onTapped: SettingsService.set(root._prefix + card.widgetKey + ".visible", false) }
        }
        Rectangle {
            visible: DesktopWidgetService.editMode
            anchors { left: parent.left; bottom: parent.bottom; margins: 8 }
            implicitWidth: _editLabel.implicitWidth + 20; implicitHeight: 27; radius: 14
            color: Qt.rgba(card.accent.r, card.accent.g, card.accent.b, 0.16)
            Row { anchors.centerIn: parent; spacing: 6
                Text { text: "󰆾"; rotation: 45; color: card.accent; font.family: ThemeManager.fontFamily; font.pixelSize: 11 }
                Text { id: _editLabel; text: card.title; color: ThemeManager.onSurface; font.family: ThemeManager.fontFamily; font.pixelSize: 9; font.weight: Font.DemiBold }
            }
        }
    }

    component WidgetToggle: Rectangle {
        required property string label
        required property string setting
        property bool defaultVisible: false
        readonly property bool selected: SettingsService.get(root._prefix + setting + ".visible", defaultVisible)
        implicitWidth: _label.implicitWidth + 24; implicitHeight: 30; radius: 15
        color: selected ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.19) : ThemeManager.surfaceContainer
        border.width: 1; border.color: selected ? Qt.rgba(ThemeManager.primary.r, ThemeManager.primary.g, ThemeManager.primary.b, 0.34) : Qt.rgba(ThemeManager.onSurface.r, ThemeManager.onSurface.g, ThemeManager.onSurface.b, 0.08)
        Text { id: _label; anchors.centerIn: parent; text: label; color: parent.selected ? ThemeManager.primary : ThemeManager.onSurfaceVariant; font.family: ThemeManager.fontFamily; font.pixelSize: 10; font.weight: parent.selected ? Font.DemiBold : Font.Normal }
        TapHandler { onTapped: SettingsService.toggle(root._prefix + setting + ".visible", parent.defaultVisible) }
    }
}
