import QtQuick 2.0
import SddmComponents 2.0

Rectangle {
    id: root
    width: 1920
    height: 1080
    color: "#11111b"

    property string currentUser: userModel.lastUser
    property bool loginFailed: false
    property string timeText: ""
    property string dateText: ""

    property int sessionIndex: {
        for (var i = 0; i < sessionModel.rowCount(); i++) {
            var name = (sessionModel.data(sessionModel.index(i, 0), Qt.DisplayRole) || "").toString()
            if (name.indexOf("Nodalix") !== -1)
                return i
        }
        return sessionModel.lastIndex
    }

    function updateClock() {
        var now = new Date()
        var hh = now.getHours().toString()
        var mm = now.getMinutes().toString()
        if (hh.length < 2) hh = "0" + hh
        if (mm.length < 2) mm = "0" + mm
        timeText = hh + ":" + mm

        var days = ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"]
        var months = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"]
        dateText = days[now.getDay()] + ", " + now.getDate() + " de " + months[now.getMonth()]
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: root.updateClock()
    }

    Connections {
        target: sddm

        function onLoginFailed() {
            root.loginFailed = true
            password.text = ""
            password.forceActiveFocus()
        }

        function onLoginSucceeded() {
            root.loginFailed = false
        }
    }

    Rectangle {
        width: parent.width * 0.7
        height: parent.height * 0.55
        radius: width
        color: "#89b4fa"
        opacity: 0.08
        anchors.horizontalCenter: parent.horizontalCenter
        y: -height * 0.42
    }

    Rectangle {
        width: parent.width * 0.46
        height: parent.height * 0.46
        radius: width
        color: "#cba6f7"
        opacity: 0.055
        anchors.horizontalCenter: parent.horizontalCenter
        y: parent.height - height * 0.30
    }

    Column {
        anchors.centerIn: parent
        spacing: 20
        width: 620

        Text {
            text: root.timeText
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#ffffff"
            font.family: "JetBrainsMono Nerd Font"
            font.pixelSize: 92
            font.bold: true
        }

        Text {
            text: root.dateText
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#cdd6f4"
            opacity: 0.72
            font.family: "JetBrainsMono Nerd Font"
            font.pixelSize: 18
        }

        Item {
            width: 1
            height: 20
        }

        Rectangle {
            width: 520
            height: 238
            radius: 28
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#181825"
            opacity: 0.94
            border.width: 1
            border.color: root.loginFailed ? "#f38ba8" : "#89b4fa"

            Column {
                anchors.fill: parent
                anchors.margins: 28
                spacing: 13

                Text {
                    text: "  Nodalix OS"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: "#ffffff"
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 25
                    font.bold: true
                }

                Text {
                    text: root.currentUser ? root.currentUser : "dani"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: "#a6adc8"
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 13
                }

                Rectangle {
                    width: parent.width
                    height: 58
                    radius: 29
                    color: "#1e1e2e"
                    border.width: 2
                    border.color: root.loginFailed ? "#f38ba8" : "#89b4fa"

                    Text {
                        anchors.centerIn: parent
                        visible: password.text.length === 0
                        text: "Contraseña"
                        color: "#cdd6f4"
                        opacity: 0.50
                        font.family: "JetBrainsMono Nerd Font"
                        font.pixelSize: 15
                    }

                    TextInput {
                        id: password
                        anchors.fill: parent
                        anchors.leftMargin: 24
                        anchors.rightMargin: 24
                        verticalAlignment: TextInput.AlignVCenter
                        horizontalAlignment: TextInput.AlignHCenter
                        echoMode: TextInput.Password
                        passwordCharacter: "•"
                        color: "#f5f5ff"
                        selectionColor: "#89b4fa"
                        selectedTextColor: "#11111b"
                        font.family: "JetBrainsMono Nerd Font"
                        font.pixelSize: 18
                        focus: true

                        onTextChanged: root.loginFailed = false

                        Keys.onPressed: {
                            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                                sddm.login(root.currentUser, password.text, root.sessionIndex)
                                event.accepted = true
                            }
                        }
                    }
                }

                Text {
                    text: root.loginFailed ? "Contraseña incorrecta" : "Pulsa Enter para iniciar sesión"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: root.loginFailed ? "#f38ba8" : "#cdd6f4"
                    opacity: root.loginFailed ? 1.0 : 0.62
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 12
                }

                Text {
                    text: "Sesión: Nodalix"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: "#89b4fa"
                    opacity: 0.75
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 11
                }
            }
        }
    }

    Text {
        text: "Nodalix login powered by SDDM"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 28
        color: "#cdd6f4"
        opacity: 0.35
        font.family: "JetBrainsMono Nerd Font"
        font.pixelSize: 11
    }

    Component.onCompleted: {
        root.updateClock()
        password.forceActiveFocus()
    }
}
