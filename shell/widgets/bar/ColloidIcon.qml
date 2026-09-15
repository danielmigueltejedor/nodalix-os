import QtQuick
import Qt5Compat.GraphicalEffects
import "../../theme"
import "../../services"

Item {
    id: root

    property string iconName: ""
    // Keep the icon-name and font-size API used by existing personal bar widgets.
    property string text: ""
    property font font: Qt.font({ pixelSize: 16 })
    property string group: "status"
    property string fallbackGlyph: text.length <= 2 ? text : ""
    property color color: ThemeManager.onSurface
    property int iconSize: font.pixelSize > 0 ? font.pixelSize : 16
    property int iconPadding: 0

    readonly property var _aliases: ({
        "preferences-desktop-apps-symbolic": "application-menu-symbolic",
        "notifications-symbolic": "notification-new-symbolic",
        "notifications-disabled-symbolic": "notification-disabled-symbolic",
        "network-wireless-disconnected-symbolic": "network-wireless-offline-symbolic",
        "network-wireless-signal-excellent-symbolic": "nm-signal-100-symbolic",
        "network-wireless-signal-good-symbolic": "nm-signal-75-symbolic",
        "network-wireless-signal-ok-symbolic": "nm-signal-50-symbolic",
        "network-wireless-signal-weak-symbolic": "nm-signal-25-symbolic",
        "network-wireless-signal-none-symbolic": "nm-signal-0-symbolic",
        "bluetooth-symbolic": "bluetooth-active-symbolic",
        "microphone-sensitivity-high-symbolic": "audio-input-microphone-high-symbolic",
        "microphone-sensitivity-muted-symbolic": "audio-input-microphone-muted-symbolic",
        "battery-level-100-symbolic": "battery-full-symbolic",
        "battery-level-100-charging-symbolic": "battery-full-charging-symbolic",
        "battery-level-40-charging-symbolic": "battery-level-30-charging-symbolic",
        "battery-level-0-symbolic": "battery-caution-symbolic"
    })

    readonly property var _glyphSpecs: ({
        "󰀄": { name: "system-users-symbolic", group: "apps" },
        "󰀻": { name: "application-menu-symbolic", group: "actions" },
        "󰂚": { name: "notification-new-symbolic", group: "status" },
        "󰂛": { name: "notification-disabled-symbolic", group: "status" },
        "󰂯": { name: "bluetooth-active-symbolic", group: "status" },
        "󰂱": { name: "bluetooth-paired-symbolic", group: "status" },
        "󰂲": { name: "bluetooth-disabled-symbolic", group: "status" },
        "󰃠": { name: "brightness-high-symbolic", group: "status" },
        "󰄜": { name: "phonelink-symbolic", group: "devices" },
        "󰄬": { name: "object-select-symbolic", group: "actions" },
        "󰅂": { name: "go-next-symbolic", group: "actions" },
        "󰅁": { name: "go-previous-symbolic", group: "actions" },
        "󰅖": { name: "window-close-symbolic", group: "actions" },
        "󰅀": { name: "go-next-symbolic", group: "actions" },
        "󰅃": { name: "go-previous-symbolic", group: "actions" },
        "󰇚": { name: "network-transmit-symbolic", group: "status" },
        "󰉼": { name: "dark-mode-symbolic", group: "actions" },
        "󰍬": { name: "audio-input-microphone-high-symbolic", group: "status" },
        "󰍭": { name: "audio-input-microphone-muted-symbolic", group: "status" },
        "󰍹": { name: "preferences-desktop-display-symbolic", group: "apps" },
        "󰍃": { name: "system-log-out-symbolic", group: "actions" },
        "󰏖": { name: "application-menu-symbolic", group: "actions" },
        "󰏤": { name: "media-playback-pause-symbolic", group: "actions" },
        "󰐊": { name: "media-playback-start-symbolic", group: "actions" },
        "󰐥": { name: "system-shutdown-symbolic", group: "status" },
        "󰒃": { name: "system-lock-screen-symbolic", group: "actions" },
        "󰌾": { name: "system-lock-screen-symbolic", group: "actions" },
        "󰒲": { name: "media-playback-pause-symbolic", group: "actions" },
        "󰑙": { name: "system-reboot-symbolic", group: "actions" },
        "󰒓": { name: "preferences-system-symbolic", group: "apps" },
        "󰒮": { name: "media-skip-backward-symbolic", group: "actions" },
        "󰒭": { name: "media-skip-forward-symbolic", group: "actions" },
        "󰕾": { name: "audio-volume-high-symbolic", group: "status" },
        "󰕿": { name: "audio-volume-low-symbolic", group: "status" },
        "󰖀": { name: "audio-volume-medium-symbolic", group: "status" },
        "󰝟": { name: "audio-volume-muted-symbolic", group: "status" },
        "󰓃": { name: "audio-volume-high-symbolic", group: "status" },
        "󰖂": { name: "network-vpn-symbolic", group: "status" },
        "󰖩": { name: "nm-signal-100-symbolic", group: "status" },
        "󰖪": { name: "network-wireless-offline-symbolic", group: "status" },
        "󰚰": { name: "network-receive-symbolic", group: "status" },
        "󰕮": { name: "go-home-symbolic", group: "actions" },
        "󰎈": { name: "multimedia-volume-control-symbolic", group: "apps" },
        "󰔎": { name: "dark-mode-symbolic", group: "actions" },
        "󰹑": { name: "screenshooter-symbolic", group: "actions" },
        "󰩭": { name: "screenshooter-symbolic", group: "actions" },
        "󰝚": { name: "multimedia-volume-control-symbolic", group: "apps" },
        "󰜉": { name: "view-refresh-symbolic", group: "actions" },
        "󰤄": { name: "power-profile-power-saver-symbolic", group: "status" },
        "󰒙": { name: "system-lock-screen-symbolic", group: "actions" },
        "󰥔": { name: "clock-app-symbolic", group: "apps" },
        "󰾆": { name: "power-profile-power-saver-symbolic", group: "status" },
        "󰾅": { name: "power-profile-balanced-symbolic", group: "status" },
        "󰓅": { name: "power-profile-performance-symbolic", group: "status" },
        "󰆴": { name: "edit-delete-symbolic", group: "actions" },
        "󰈉": { name: "view-reveal-symbolic", group: "actions" },
        "󰈈": { name: "view-conceal-symbolic", group: "actions" }
    })

    readonly property string _name: iconName || (_glyphSpecs[text]?.name ?? _aliases[text]
                                             ?? (text.endsWith("-symbolic") ? text : ""))
    readonly property string _group: iconName ? group
                                      : (_glyphSpecs[text]?.group ?? (_name === "application-menu-symbolic" ? "actions" : group))

    implicitWidth: iconSize
    implicitHeight: iconSize

    readonly property string iconFile: Paths.configDir + "/assets/icons/colloid-bold/"
                                       + _group + "/" + _name + ".svg"
    readonly property bool iconLoaded: mask.status === Image.Ready

    Image {
        id: mask
        anchors.fill: parent
        anchors.margins: root.iconPadding
        source: root._name ? root.iconFile : ""
        sourceSize: Qt.size(root.iconSize * 3, root.iconSize * 3)
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
        visible: false
    }

    ColorOverlay {
        anchors.fill: mask
        source: mask
        color: root.color
        cached: true
        visible: mask.status === Image.Ready
    }

    Text {
        anchors.centerIn: parent
        visible: mask.status !== Image.Ready
        text: root.fallbackGlyph
        color: root.color
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: root.iconSize
    }
}
