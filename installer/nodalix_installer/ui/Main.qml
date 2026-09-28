import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root

    visible: true
    width: 1180
    height: 760
    minimumWidth: 980
    minimumHeight: 660

    title: "Instalar Nodalix"
    color: "#0b0d12"

    property int step: 0
    property var selectedDisk: null

    property string selectedWifi: ""
    property bool selectedWifiSecured: false
    property string wifiPassword: ""

    readonly property var localeOptions: [
        { label: "Español (España)", value: "es_ES.UTF-8" },
        { label: "English (United States)", value: "en_US.UTF-8" },
        { label: "English (United Kingdom)", value: "en_GB.UTF-8" },
        { label: "Français (France)", value: "fr_FR.UTF-8" },
        { label: "Deutsch (Deutschland)", value: "de_DE.UTF-8" },
        { label: "Italiano (Italia)", value: "it_IT.UTF-8" },
        { label: "Português (Portugal)", value: "pt_PT.UTF-8" }
    ]

    readonly property var keyboardOptions: [
        { label: "Español", value: "es" },
        { label: "English (US)", value: "us" },
        { label: "English (UK)", value: "gb" },
        { label: "Français", value: "fr" },
        { label: "Deutsch", value: "de" },
        { label: "Italiano", value: "it" },
        { label: "Português", value: "pt" }
    ]

    readonly property var timezoneOptions: [
        { label: "Madrid", value: "Europe/Madrid" },
        { label: "London", value: "Europe/London" },
        { label: "Paris", value: "Europe/Paris" },
        { label: "Berlin", value: "Europe/Berlin" },
        { label: "Rome", value: "Europe/Rome" },
        { label: "Lisbon", value: "Europe/Lisbon" },
        { label: "New York", value: "America/New_York" },
        { label: "Los Angeles", value: "America/Los_Angeles" }
    ]


    onStepChanged: {
        if (step === 1)
            installer.refresh_wifi()
    }

    property string fullName: ""
    property string username: ""
    property string hostname: "nodalix"
    property string password: ""

    readonly property color bgColor: "#0b0d12"
    readonly property color surface: "#12151c"
    readonly property color surfaceHover: "#181c25"
    readonly property color border: "#252a35"
    readonly property color textPrimary: "#f5f7fa"
    readonly property color textSecondary: "#949baa"
    readonly property color accent: "#72a7ff"
    readonly property color accentSoft: "#17243a"
    readonly property color success: "#67d391"

    function networkConnected() {
        var state = installer.network.toLowerCase()

        return state === "conectado"
            || state === "connected"
            || state.indexOf("conectado") >= 0
            || state.indexOf("connected") >= 0
    }

    function diskName(disk) {
        if (!disk)
            return "Ningún disco"

        if (disk.model && disk.model.length > 0)
            return disk.model

        return disk.path
    }

    component PrimaryButton: Button {
        id: control

        implicitHeight: 48
        implicitWidth: 150

        font.pixelSize: 14
        font.weight: Font.DemiBold

        background: Rectangle {
            radius: 12

            color: control.enabled
                ? (control.down ? "#5791ee" : root.accent)
                : "#313642"

            Behavior on color {
                ColorAnimation { duration: 100 }
            }
        }

        contentItem: Text {
            text: control.text

            color: control.enabled
                ? "#07101d"
                : "#767d8a"

            font: control.font

            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    component SecondaryButton: Button {
        id: control

        implicitHeight: 48
        implicitWidth: 125

        font.pixelSize: 14
        font.weight: Font.DemiBold

        background: Rectangle {
            radius: 12

            color: control.down
                ? "#202530"
                : root.surface

            border.width: 1
            border.color: root.border
        }

        contentItem: Text {
            text: control.text
            color: control.enabled ? root.textPrimary : "#626875"
            font: control.font

            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    component ModernField: TextField {
        id: control

        implicitHeight: 50

        color: root.textPrimary
        placeholderTextColor: "#686f7d"

        font.pixelSize: 14

        leftPadding: 16
        rightPadding: 16

        background: Rectangle {
            radius: 12

            color: "#0e1117"

            border.width: control.activeFocus ? 2 : 1
            border.color: control.activeFocus
                ? root.accent
                : root.border
        }
    }

    component ModernCombo: ComboBox {
        id: control

        implicitHeight: 50

        textRole: "label"
        valueRole: "value"

        font.pixelSize: 14

        leftPadding: 15
        rightPadding: 40

        contentItem: Text {
            text: control.displayText

            color: root.textPrimary

            font: control.font

            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        indicator: Label {
            anchors.right: parent.right
            anchors.rightMargin: 15
            anchors.verticalCenter: parent.verticalCenter

            text: "⌄"

            color: root.textSecondary

            font.pixelSize: 17
        }

        background: Rectangle {
            radius: 11

            color: control.pressed
                ? "#151921"
                : "#0e1117"

            border.width: control.activeFocus ? 1.5 : 1

            border.color: control.activeFocus
                ? root.accent
                : root.border
        }

        delegate: ItemDelegate {
            id: option

            required property var modelData

            width: ListView.view.width
            height: 44

            leftPadding: 13
            rightPadding: 13

            contentItem: Text {
                text: modelData.label

                color: root.textPrimary

                font.pixelSize: 13

                verticalAlignment: Text.AlignVCenter
            }

            background: Rectangle {
                radius: 8

                color: option.highlighted
                    ? "#1c2635"
                    : "transparent"
            }
        }

        popup: Popup {
            y: control.height + 6

            width: control.width

            padding: 6

            implicitHeight: Math.min(contentItem.implicitHeight + 12, 280)

            background: Rectangle {
                radius: 12

                color: "#11151c"

                border.width: 1
                border.color: root.border
            }

            contentItem: ListView {
                clip: true

                implicitHeight: contentHeight

                model: control.popup.visible
                    ? control.delegateModel
                    : null

                currentIndex: control.highlightedIndex

                ScrollIndicator.vertical: ScrollIndicator {}
            }
        }
    }

    component PageHeader: ColumnLayout {
        property alias title: titleLabel.text
        property alias description: descriptionLabel.text

        spacing: 8

        Label {
            id: titleLabel

            color: root.textPrimary

            font.pixelSize: 31
            font.weight: Font.DemiBold
        }

        Label {
            id: descriptionLabel

            Layout.maximumWidth: 650

            color: root.textSecondary

            font.pixelSize: 15

            wrapMode: Text.WordWrap
        }
    }

    component Card: Rectangle {
        radius: 18

        color: root.surface

        border.width: 1
        border.color: root.border
    }

    Rectangle {
        anchors.fill: parent
        color: root.bgColor

        Rectangle {
            width: 440
            height: 440

            radius: 220

            x: -220
            y: -250

            color: "#245da8"
            opacity: 0.10
        }

        Rectangle {
            width: 500
            height: 500

            radius: 250

            anchors.right: parent.right
            anchors.bottom: parent.bottom

            anchors.rightMargin: -270
            anchors.bottomMargin: -330

            color: "#224d88"
            opacity: 0.08
        }
    }

    ColumnLayout {
        anchors.fill: parent

        anchors.leftMargin: 44
        anchors.rightMargin: 44
        anchors.topMargin: 30
        anchors.bottomMargin: 30

        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 58

            spacing: 14

            Image {
                Layout.preferredWidth: 42
                Layout.preferredHeight: 42
                source: Qt.resolvedUrl("../assets/nodalix.png")
                sourceSize.width: 84
                sourceSize.height: 84
                fillMode: Image.PreserveAspectFit
                smooth: true
                mipmap: true
            }

            ColumnLayout {
                spacing: 1

                Label {
                    text: "Nodalix"
                    color: root.textPrimary

                    font.pixelSize: 18
                    font.weight: Font.DemiBold
                }

                Label {
                    text: "Instalador"
                    color: root.textSecondary

                    font.pixelSize: 12
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Rectangle {
                implicitWidth: networkRow.implicitWidth + 28
                height: 38

                radius: 19

                color: root.networkConnected()
                    ? "#10251b"
                    : "#251a17"

                border.width: 1
                border.color: root.networkConnected()
                    ? "#245f3d"
                    : "#624036"

                RowLayout {
                    id: networkRow

                    anchors.centerIn: parent

                    spacing: 8

                    Rectangle {
                        width: 8
                        height: 8

                        radius: 4

                        color: root.networkConnected()
                            ? root.success
                            : "#e58a74"
                    }

                    Label {
                        text: installer.network

                        color: root.networkConnected()
                            ? "#a8e7be"
                            : "#e9b3a5"

                        font.pixelSize: 12
                        font.weight: Font.Medium
                    }
                }
            }
        }

        Item {
            Layout.preferredHeight: 22
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredHeight: 42

            spacing: 10

            Repeater {
                model: [
                    "Bienvenida",
                    "Conexión",
                    "Disco",
                    "Cuenta",
                    "Resumen"
                ]

                delegate: RowLayout {
                    required property string modelData
                    required property int index

                    spacing: 10

                    Rectangle {
                        implicitWidth: stepContent.implicitWidth + 24
                        height: 36

                        radius: 18

                        color: root.step === index
                            ? root.accentSoft
                            : "transparent"

                        border.width: root.step === index ? 1 : 0
                        border.color: "#294b78"

                        RowLayout {
                            id: stepContent

                            anchors.centerIn: parent

                            spacing: 7

                            Rectangle {
                                width: 20
                                height: 20

                                radius: 10

                                color: root.step >= index
                                    ? root.accent
                                    : "#2b303a"

                                Label {
                                    anchors.centerIn: parent

                                    text: index + 1

                                    color: root.step >= index
                                        ? "#07101d"
                                        : "#818896"

                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }

                            Label {
                                text: modelData

                                color: root.step === index
                                    ? root.textPrimary
                                    : root.textSecondary

                                font.pixelSize: 12
                                font.weight: root.step === index
                                    ? Font.DemiBold
                                    : Font.Normal
                            }
                        }
                    }

                    Rectangle {
                        visible: index < 4

                        width: 25
                        height: 1

                        color: root.step > index
                            ? root.accent
                            : root.border
                    }
                }
            }
        }

        Item {
            Layout.preferredHeight: 28
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true

            currentIndex: root.step

            Item {
                ColumnLayout {
                    anchors.centerIn: parent

                    width: Math.min(700, parent.width - 100)

                    spacing: 24

                    Item {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 82

                        Image {
                            anchors.centerIn: parent
                            width: 124
                            height: 124
                            source: Qt.resolvedUrl("../assets/nodalix.png")
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            mipmap: true
                        }
                    }

                    Label {
                        Layout.fillWidth: true

                        text: "Bienvenido a Nodalix"

                        color: root.textPrimary

                        font.pixelSize: 38
                        font.weight: Font.DemiBold

                        horizontalAlignment: Text.AlignHCenter
                    }

                    Label {
                        Layout.fillWidth: true

                        text: "Configura tu nuevo sistema en unos minutos. Te guiaremos por la conexión, el disco y la creación de tu cuenta."

                        color: root.textSecondary

                        font.pixelSize: 16

                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Item {
                        Layout.preferredHeight: 12
                    }

                    Card {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 88

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 20

                            spacing: 16

                            Rectangle {
                                width: 42
                                height: 42

                                radius: 13

                                color: root.networkConnected()
                                    ? "#10251b"
                                    : "#251a17"

                                Label {
                                    anchors.centerIn: parent

                                    text: root.networkConnected() ? "✓" : "!"
                                    color: root.networkConnected()
                                        ? root.success
                                        : "#e58a74"

                                    font.pixelSize: 18
                                    font.bold: true
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true

                                spacing: 3

                                Label {
                                    text: root.networkConnected()
                                        ? "Conexión preparada"
                                        : "Sin conexión"

                                    color: root.textPrimary

                                    font.pixelSize: 15
                                    font.weight: Font.DemiBold
                                }

                                Label {
                                    text: root.networkConnected()
                                        ? "Nodalix tiene acceso a Internet."
                                        : "Podrás configurar la conexión en el siguiente paso."

                                    color: root.textSecondary

                                    font.pixelSize: 12
                                }
                            }
                        }
                    }
                }
            }

            Item {
                ColumnLayout {
                    anchors.fill: parent

                    spacing: 24

                    PageHeader {
                        title: "Conexión a Internet"

                        description: root.networkConnected()
                            ? "Tu equipo está conectado. Puedes continuar con la instalación."
                            : "Conecta el equipo mediante Ethernet o configura una red Wi-Fi."
                    }

                    Card {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 120

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 22

                            spacing: 18

                            Rectangle {
                                width: 52
                                height: 52

                                radius: 16

                                color: root.networkConnected()
                                    ? "#10251b"
                                    : "#251a17"

                                Label {
                                    anchors.centerIn: parent

                                    text: root.networkConnected() ? "✓" : "○"

                                    color: root.networkConnected()
                                        ? root.success
                                        : "#e58a74"

                                    font.pixelSize: 21
                                    font.bold: true
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true

                                spacing: 4

                                Label {
                                    text: root.networkConnected()
                                        ? "Conectado"
                                        : "No conectado"

                                    color: root.textPrimary

                                    font.pixelSize: 17
                                    font.weight: Font.DemiBold
                                }

                                Label {
                                    text: installer.network

                                    color: root.textSecondary

                                    font.pixelSize: 13
                                }
                            }

                            SecondaryButton {
                                text: "Actualizar"

                                onClicked: installer.refresh_network()
                            }
                        }
                    }

                    Card {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 24

                            spacing: 10

                            RowLayout {
                                Layout.fillWidth: true

                                Label {
                                    text: "Redes Wi-Fi"

                                    color: root.textPrimary

                                    font.pixelSize: 17
                                    font.weight: Font.DemiBold
                                }

                                Item {
                                    Layout.fillWidth: true
                                }

                                SecondaryButton {
                                    text: installer.wifiBusy
                                        ? "Buscando…"
                                        : "Actualizar"

                                    enabled: !installer.wifiBusy

                                    onClicked: installer.refresh_wifi()
                                }
                            }

                            Label {
                                visible: installer.wifiError.length > 0

                                Layout.fillWidth: true

                                text: installer.wifiError

                                color: "#e58a74"

                                font.pixelSize: 13

                                wrapMode: Text.WordWrap
                            }

                            BusyIndicator {
                                visible: installer.wifiBusy
                                running: visible

                                Layout.alignment: Qt.AlignHCenter
                            }

                            ScrollView {
                                visible: !installer.wifiBusy
                                    && installer.wifiError.length === 0

                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                clip: true

                                Column {
                                    width: parent.width

                                    spacing: 8

                                    Repeater {
                                        model: installer.wifiNetworks

                                        delegate: Rectangle {
                                            required property var modelData

                                            width: parent.width
                                            height: 68

                                            radius: 14

                                            color: modelData.active
                                                ? "#10251b"
                                                : root.selectedWifi === modelData.ssid
                                                    ? root.accentSoft
                                                    : "#0e1117"

                                            border.width: 1

                                            border.color: modelData.active
                                                ? "#245f3d"
                                                : root.selectedWifi === modelData.ssid
                                                    ? root.accent
                                                    : root.border

                                            RowLayout {
                                                anchors.fill: parent
                                                anchors.margins: 14

                                                spacing: 14

                                                ColumnLayout {
                                                    Layout.fillWidth: true

                                                    spacing: 3

                                                    Label {
                                                        text: modelData.ssid

                                                        color: root.textPrimary

                                                        font.pixelSize: 14
                                                        font.weight: Font.DemiBold
                                                    }

                                                    Label {
                                                        text: modelData.active
                                                            ? "Conectada"
                                                            : modelData.secured
                                                                ? "Red protegida"
                                                                : "Red abierta"

                                                        color: modelData.active
                                                            ? root.success
                                                            : root.textSecondary

                                                        font.pixelSize: 11
                                                    }
                                                }

                                                Label {
                                                    text: modelData.signal >= 75
                                                        ? "▂▄▆█"
                                                        : modelData.signal >= 50
                                                            ? "▂▄▆"
                                                            : modelData.signal >= 25
                                                                ? "▂▄"
                                                                : "▂"

                                                    color: modelData.signal >= 60
                                                        ? root.textPrimary
                                                        : root.textSecondary

                                                    font.pixelSize: 15
                                                }

                                                Label {
                                                    text: modelData.signal + "%"

                                                    color: root.textSecondary

                                                    font.pixelSize: 11
                                                }

                                                SecondaryButton {
                                                    visible: !modelData.active

                                                    text: "Conectar"

                                                    implicitWidth: 105
                                                    implicitHeight: 40

                                                    onClicked: {
                                                        if (modelData.secured) {
                                                            root.selectedWifi = modelData.ssid
                                                            root.selectedWifiSecured = true
                                                            root.wifiPassword = ""
                                                        } else {
                                                            root.selectedWifi = ""
                                                            root.selectedWifiSecured = false

                                                            installer.connect_wifi(
                                                                modelData.ssid,
                                                                ""
                                                            )
                                                        }
                                                    }
                                                }

                                                Rectangle {
                                                    visible: modelData.active

                                                    implicitWidth: 90
                                                    height: 34

                                                    radius: 17

                                                    color: "#163424"

                                                    Label {
                                                        anchors.centerIn: parent

                                                        text: "Conectada"

                                                        color: "#9ce6b7"

                                                        font.pixelSize: 11
                                                        font.weight: Font.DemiBold
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                visible: root.selectedWifi.length > 0
                                    && root.selectedWifiSecured

                                Layout.fillWidth: true
                                Layout.preferredHeight: 82

                                radius: 14

                                color: root.accentSoft

                                border.width: 1
                                border.color: "#294b78"

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.margins: 14

                                    spacing: 12

                                    ColumnLayout {
                                        Layout.preferredWidth: 180

                                        spacing: 2

                                        Label {
                                            text: root.selectedWifi

                                            color: root.textPrimary

                                            font.pixelSize: 13
                                            font.weight: Font.DemiBold

                                            elide: Text.ElideRight
                                        }

                                        Label {
                                            text: "Introduce la contraseña"

                                            color: root.textSecondary

                                            font.pixelSize: 11
                                        }
                                    }

                                    ModernField {
                                        Layout.fillWidth: true

                                        placeholderText: "Contraseña Wi-Fi"

                                        echoMode: TextInput.Password

                                        text: root.wifiPassword

                                        onTextChanged: root.wifiPassword = text

                                        onAccepted: {
                                            if (root.wifiPassword.length > 0) {
                                                installer.connect_wifi(
                                                    root.selectedWifi,
                                                    root.wifiPassword
                                                )

                                                root.selectedWifi = ""
                                                root.wifiPassword = ""
                                            }
                                        }
                                    }

                                    PrimaryButton {
                                        text: "Conectar"

                                        implicitWidth: 110

                                        enabled: root.wifiPassword.length > 0
                                            && !installer.wifiBusy

                                        onClicked: {
                                            installer.connect_wifi(
                                                root.selectedWifi,
                                                root.wifiPassword
                                            )

                                            root.selectedWifi = ""
                                            root.wifiPassword = ""
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Item {
                ColumnLayout {
                    anchors.fill: parent

                    spacing: 22

                    PageHeader {
                        title: "Selecciona el disco"
                        description: "Elige dónde quieres instalar Nodalix. Todavía no se realizará ningún cambio."
                    }

                    ScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        clip: true

                        Column {
                            width: parent.width

                            spacing: 12

                            Repeater {
                                model: installer.disks

                                delegate: Rectangle {
                                    required property var modelData

                                    width: parent.width
                                    height: 92

                                    radius: 16

                                    color: root.selectedDisk
                                        && root.selectedDisk.path === modelData.path
                                        ? root.accentSoft
                                        : root.surface

                                    border.width: root.selectedDisk
                                        && root.selectedDisk.path === modelData.path
                                        ? 2
                                        : 1

                                    border.color: root.selectedDisk
                                        && root.selectedDisk.path === modelData.path
                                        ? root.accent
                                        : root.border

                                    MouseArea {
                                        anchors.fill: parent

                                        cursorShape: Qt.PointingHandCursor

                                        onClicked: {
                                            root.selectedDisk = modelData
                                        }
                                    }

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.margins: 18

                                        spacing: 16

                                        Rectangle {
                                            width: 48
                                            height: 48

                                            radius: 14

                                            color: "#1a1f29"

                                            Label {
                                                anchors.centerIn: parent

                                                text: "▣"

                                                color: root.textSecondary
                                                font.pixelSize: 20
                                            }
                                        }

                                        ColumnLayout {
                                            Layout.fillWidth: true

                                            spacing: 4

                                            Label {
                                                text: modelData.model || "Disco"

                                                color: root.textPrimary

                                                font.pixelSize: 16
                                                font.weight: Font.DemiBold
                                            }

                                            Label {
                                                text: modelData.path

                                                color: root.textSecondary

                                                font.pixelSize: 12
                                            }
                                        }

                                        Label {
                                            text: modelData.size

                                            color: root.textPrimary

                                            font.pixelSize: 14
                                            font.weight: Font.Medium
                                        }

                                        Rectangle {
                                            width: 22
                                            height: 22

                                            radius: 11

                                            color: root.selectedDisk
                                                && root.selectedDisk.path === modelData.path
                                                ? root.accent
                                                : "transparent"

                                            border.width: 2
                                            border.color: root.selectedDisk
                                                && root.selectedDisk.path === modelData.path
                                                ? root.accent
                                                : "#4b5260"

                                            Label {
                                                visible: root.selectedDisk
                                                    && root.selectedDisk.path === modelData.path

                                                anchors.centerIn: parent

                                                text: "✓"
                                                color: "#07101d"

                                                font.pixelSize: 11
                                                font.bold: true
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Item {
                ColumnLayout {
                    anchors.fill: parent

                    spacing: 22

                    PageHeader {
                        title: "Crea tu cuenta"
                        description: "Esta será la cuenta principal de tu instalación de Nodalix."
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        spacing: 18

                        Card {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 24

                                spacing: 11

                                Label {
                                    text: "Nombre"

                                    color: root.textSecondary
                                    font.pixelSize: 12
                                }

                                ModernField {
                                    Layout.fillWidth: true

                                    placeholderText: "Daniel"

                                    text: root.fullName

                                    onTextChanged: root.fullName = text
                                }

                                Label {
                                    text: "Usuario"

                                    color: root.textSecondary
                                    font.pixelSize: 12
                                }

                                ModernField {
                                    Layout.fillWidth: true

                                    placeholderText: "daniel"

                                    text: root.username

                                    onTextChanged: root.username = text
                                }

                                Label {
                                    text: "Contraseña"

                                    color: root.textSecondary
                                    font.pixelSize: 12
                                }

                                ModernField {
                                    Layout.fillWidth: true

                                    placeholderText: "Contraseña"

                                    echoMode: TextInput.Password

                                    text: root.password

                                    onTextChanged: root.password = text
                                }

                                Item {
                                    Layout.fillHeight: true
                                }
                            }
                        }

                        Card {
                            Layout.preferredWidth: 330
                            Layout.fillHeight: true

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 24

                                spacing: 11

                                Label {
                                    text: "Nombre del equipo"

                                    color: root.textSecondary
                                    font.pixelSize: 12
                                }

                                ModernField {
                                    Layout.fillWidth: true

                                    text: root.hostname

                                    onTextChanged: root.hostname = text
                                }

                                Label {
                                    text: "Idioma y región"

                                    color: root.textSecondary

                                    font.pixelSize: 12
                                }

                                ModernCombo {
                                    Layout.fillWidth: true

                                    model: root.localeOptions
                                    currentIndex: 0

                                    onActivated: {
                                        installer.locale = currentValue
                                    }
                                }

                                Label {
                                    text: "Zona horaria"

                                    color: root.textSecondary

                                    font.pixelSize: 12
                                }

                                ModernCombo {
                                    Layout.fillWidth: true

                                    model: root.timezoneOptions
                                    currentIndex: 0

                                    onActivated: {
                                        installer.timezone = currentValue
                                    }
                                }

                                Label {
                                    text: "Distribución del teclado"

                                    color: root.textSecondary

                                    font.pixelSize: 12
                                }

                                ModernCombo {
                                    Layout.fillWidth: true

                                    model: root.keyboardOptions
                                    currentIndex: 0

                                    onActivated: {
                                        installer.keyboardLayout = currentValue
                                    }
                                }

                                Label {
                                    text: "Prueba de teclado"

                                    color: root.textSecondary

                                    font.pixelSize: 12
                                }

                                ModernField {
                                    Layout.fillWidth: true

                                    placeholderText: "Escribe aquí para comprobar el teclado…"
                                }

                                Item {
                                    Layout.fillHeight: true
                                }
                            }
                        }
                    }
                }
            }

            Item {
                ColumnLayout {
                    anchors.fill: parent

                    spacing: 22

                    PageHeader {
                        title: "Todo preparado"
                        description: "Revisa la configuración antes de comenzar la instalación."
                    }

                    Card {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 26

                            spacing: 0

                            Repeater {
                                model: [
                                    {
                                        title: "Destino",
                                        value: root.selectedDisk
                                            ? root.diskName(root.selectedDisk)
                                            : "Sin seleccionar"
                                    },
                                    {
                                        title: "Usuario",
                                        value: root.username.length > 0
                                            ? root.username
                                            : "Sin configurar"
                                    },
                                    {
                                        title: "Equipo",
                                        value: root.hostname
                                    },
                                    {
                                        title: "Conexión",
                                        value: installer.network
                                    },
                                    {
                                        title: "Idioma",
                                        value: installer.locale + " · " + installer.timezone + " · teclado " + installer.keyboardLayout
                                    }
                                ]

                                delegate: Item {
                                    required property var modelData

                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 62

                                    RowLayout {
                                        anchors.fill: parent

                                        Label {
                                            text: modelData.title

                                            color: root.textSecondary

                                            font.pixelSize: 13

                                            Layout.preferredWidth: 130
                                        }

                                        Label {
                                            text: modelData.value

                                            color: root.textPrimary

                                            font.pixelSize: 14
                                            font.weight: Font.Medium

                                            Layout.fillWidth: true
                                        }
                                    }

                                    Rectangle {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.bottom: parent.bottom

                                        height: 1

                                        color: root.border
                                    }
                                }
                            }

                            Item {
                                Layout.fillHeight: true
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 72

                                radius: 14

                                color: "#161f2d"

                                border.width: 1
                                border.color: "#29415f"

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.margins: 18

                                    Label {
                                        text: "i"

                                        color: root.accent

                                        font.pixelSize: 18
                                        font.bold: true
                                    }

                                    Label {
                                        Layout.fillWidth: true

                                        text: "Todavía no hay operaciones destructivas conectadas. El siguiente paso será implementar el backend real de instalación."

                                        color: "#b7c5d8"

                                        font.pixelSize: 12

                                        wrapMode: Text.WordWrap
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Item {
            Layout.preferredHeight: 20
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 52

            SecondaryButton {
                text: "Atrás"

                visible: root.step > 0

                onClicked: root.step--
            }

            Item {
                Layout.fillWidth: true
            }

            Label {
                visible: root.step === 2 && root.selectedDisk === null

                text: "Selecciona un disco para continuar"

                color: root.textSecondary

                font.pixelSize: 12
            }

            PrimaryButton {
                text: root.step === 4
                    ? "Instalar Nodalix"
                    : "Continuar"

                enabled: root.step !== 2 || root.selectedDisk !== null

                onClicked: {
                    if (root.step < 4) {
                        root.step++
                    } else {
                        installer.setStatus("Preparando instalación…")
                    }
                }
            }
        }
    }
}
