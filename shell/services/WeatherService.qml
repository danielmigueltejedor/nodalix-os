pragma Singleton
import QtQuick
import "."

// Open-Meteo forecast over verified HTTPS. An empty location keeps the
// previous approximate-by-IP behavior; a configured city takes precedence.
QtObject {
    id: root

    readonly property string place: String(SettingsService.get("weather.location", "") ?? "").trim()
    readonly property bool useF: SettingsService.get("weather.fahrenheit", false)
    readonly property string unit: useF ? "°F" : "°C"

    property bool ok: false
    property bool loading: false
    property string lastError: ""
    property int tempC: 0
    property int feelsC: 0
    property string desc: ""
    property int humidity: 0
    property string icon: "󰖐"
    property string location: ""
    property var forecast: []

    property int _requestId: 0
    property var _coords: null
    property string _coordsKey: ""

    function conv(c) { return useF ? Math.round(c * 9 / 5 + 32) : Math.round(c) }
    readonly property int temp: conv(tempC)
    readonly property int feels: conv(feelsC)

    function _glyph(code) {
        const c = Number(code)
        if (c === 0 || c === 1) return "󰖙"
        if (c === 2 || c === 3) return "󰖕"
        if (c === 45 || c === 48) return "󰖑"
        if (c >= 95) return "󰖓"
        if ((c >= 71 && c <= 77) || c === 85 || c === 86) return "󰖘"
        if (c >= 51 && c <= 82) return "󰖗"
        return "󰖐"
    }

    function _description(code) {
        const c = Number(code)
        const es = I18n.language === "es"
        if (c === 0) return es ? "Despejado" : "Clear"
        if (c === 1 || c === 2) return es ? "Poco nuboso" : "Partly cloudy"
        if (c === 3) return es ? "Nublado" : "Cloudy"
        if (c === 45 || c === 48) return es ? "Niebla" : "Fog"
        if (c >= 95) return es ? "Tormenta" : "Thunderstorm"
        if ((c >= 71 && c <= 77) || c === 85 || c === 86) return es ? "Nieve" : "Snow"
        if (c >= 51 && c <= 57) return es ? "Llovizna" : "Drizzle"
        if (c >= 61 && c <= 82) return es ? "Lluvia" : "Rain"
        return es ? "Tiempo variable" : "Variable weather"
    }

    function _fail(id, reason) {
        if (id !== root._requestId) return
        root._deadline.stop()
        root.loading = false
        root.lastError = reason
        // Keep the last valid forecast visible on transient network failures.
    }

    function _get(url, id, done) {
        const xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function () {
            if (xhr.readyState !== XMLHttpRequest.DONE || id !== root._requestId) return
            if (xhr.status !== 200) {
                root._fail(id, "HTTP " + xhr.status)
                return
            }
            try { done(JSON.parse(xhr.responseText)) }
            catch (e) { root._fail(id, String(e)) }
        }
        xhr.onerror = function () { root._fail(id, "Network error") }
        xhr.open("GET", url)
        xhr.send()
    }

    function _forecast(coords, id) {
        const url = "https://api.open-meteo.com/v1/forecast"
            + "?latitude=" + encodeURIComponent(coords.latitude)
            + "&longitude=" + encodeURIComponent(coords.longitude)
            + "&current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code"
            + "&daily=temperature_2m_max,temperature_2m_min,weather_code"
            + "&timezone=auto&forecast_days=3"
        root._get(url, id, function (j) {
            const cc = j.current
            const d = j.daily
            if (!cc || !isFinite(Number(cc.temperature_2m))
                    || !isFinite(Number(cc.apparent_temperature))) {
                root._fail(id, "Invalid current conditions")
                return
            }
            const locale = Qt.locale(I18n.localeName)
            const days = []
            for (let i = 0; i < Math.min(3, d?.time?.length ?? 0); i++) {
                days.push({
                    day: new Date(d.time[i] + "T12:00:00").toLocaleDateString(locale, "ddd"),
                    min: Number(d.temperature_2m_min[i]),
                    max: Number(d.temperature_2m_max[i]),
                    icon: root._glyph(d.weather_code[i])
                })
            }
            root.tempC = Math.round(Number(cc.temperature_2m))
            root.feelsC = Math.round(Number(cc.apparent_temperature))
            root.humidity = Math.round(Number(cc.relative_humidity_2m ?? 0))
            root.icon = root._glyph(cc.weather_code)
            root.desc = root._description(cc.weather_code)
            root.location = coords.city
            root.forecast = days
            root.lastError = ""
            root.loading = false
            root._deadline.stop()
            root.ok = true
        })
    }

    function refresh() {
        const id = ++root._requestId
        root.loading = true
        root.lastError = ""
        root._deadline.restart()
        if (root._coords !== null && root._coordsKey === root.place) {
            root._forecast(root._coords, id)
            return
        }
        if (root.place === "") {
            root._get("https://ipwho.is/", id, function (j) {
                if (!j.success || !isFinite(Number(j.latitude))
                        || !isFinite(Number(j.longitude))) {
                    root._fail(id, "Location unavailable")
                    return
                }
                root._coords = {
                    latitude: Number(j.latitude), longitude: Number(j.longitude),
                    city: String(j.city || "")
                }
                root._coordsKey = ""
                root._forecast(root._coords, id)
            })
            return
        }
        const lang = I18n.language === "es" ? "es" : "en"
        const url = "https://geocoding-api.open-meteo.com/v1/search"
            + "?name=" + encodeURIComponent(root.place) + "&count=1&language=" + lang
        root._get(url, id, function (j) {
            const hit = j.results?.[0]
            if (!hit || !isFinite(Number(hit.latitude))
                    || !isFinite(Number(hit.longitude))) {
                root._fail(id, "City not found")
                return
            }
            root._coords = {
                latitude: Number(hit.latitude), longitude: Number(hit.longitude),
                city: String(hit.name || root.place)
            }
            root._coordsKey = root.place
            root._forecast(root._coords, id)
        })
    }

    onPlaceChanged: {
        _coords = null
        _coordsKey = ""
        refresh()
    }

    property Timer _deadline: Timer {
        interval: 12000
        onTriggered: root._fail(root._requestId, "Timeout")
    }
    property Timer _poll: Timer {
        interval: Math.max(5, SettingsService.get("weather.refreshMin", 30)) * 60000
        repeat: true
        running: true
        onTriggered: root.refresh()
    }
    Component.onCompleted: refresh()
}
