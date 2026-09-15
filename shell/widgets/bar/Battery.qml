import QtQuick
import QtQuick.Layouts
import Quickshell.Services.UPower
import "../../theme"
import "../../services"

RowLayout {
    id: root
    spacing: 4

    property var barScreen: null

    readonly property var device: UPower.displayDevice
    readonly property bool available: device !== null && device.isPresent && device.isLaptopBattery
    readonly property int  pct:   device ? Math.round(device.percentage * 100) : -1

    // State: 0=unknown 1=charging 2=discharging 3=empty 4=full 5=pending-charge
    readonly property bool charging: device ? (device.state === UPowerDeviceState.Charging ||
                                               device.state === UPowerDeviceState.PendingCharge ||
                                               device.state === UPowerDeviceState.FullyCharged) : false
    readonly property bool critical: pct >= 0 && pct <= 15 && !charging

    visible: available

    ColloidIcon {
        iconName: {
            if      (pct < 0)    return "battery-missing-symbolic"
            if      (charging)   return pct >= 90 ? "battery-full-charging-symbolic" : pct >= 50 ? "battery-good-charging-symbolic" : "battery-low-charging-symbolic"
            else if (pct >= 90)  return "battery-full-symbolic"
            else if (pct >= 70)  return "battery-good-symbolic"
            else if (pct >= 50)  return "battery-good-symbolic"
            else if (pct >= 30)  return "battery-good-symbolic"
            else if (pct >= 15)  return "battery-low-symbolic"
            else                 return "battery-caution-symbolic"
        }
        fallbackGlyph: "󰁹"
        color: critical ? ThemeManager.error : ThemeManager.onSurface
        Layout.alignment: Qt.AlignVCenter

        Behavior on color { ColorAnimation { duration: 200 } }
    }

    Text {
        text: pct >= 0 ? pct + "%" : ""
        visible: pct >= 0
        color: critical ? ThemeManager.error : ThemeManager.onSurface
        font.family: ThemeManager.fontFor(text)
        font.pixelSize: ThemeManager.fontSizeSm
        font.weight: Font.Medium
        Layout.alignment: Qt.AlignVCenter

        Behavior on color { ColorAnimation { duration: 200 } }
    }

    HoverHandler {
        onHoveredChanged: {
            const pos = root.mapToItem(null, root.width / 2, 0)
            if (hovered) {
                PopoutService.open("powerprofile", pos.x, root.barScreen)
                PopoutService.widgetHovered = true
            } else {
                PopoutService.widgetHovered = false
            }
        }
    }
}
