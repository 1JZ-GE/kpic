import QtQuick
import QtQuick.Layouts
import Qt.labs.platform
import org.kde.plasma.components as PlasmaComponents
import org.kde.kirigami as Kirigami

// popup content directly; no outer frame to avoid clipping
Item {
    id: root
    implicitWidth: 320
    implicitHeight: 440
    property var uris: []
    property int quality: 50
    property bool lossless: false
    property string format: ""
    // live job state, fed by the client via popup view
    property bool busy: false
    property int progressIndex: 0
    property int progressDone: 0
    property int progressTotal: 0
    signal compressRequested
    signal cancelRequested

    onBusyChanged: {
        if (root.busy) {
            root.progressIndex = 0
            root.progressDone = 0
            root.progressTotal = 0
        }
    }

    FileDialog {
        id: fileDialog
        fileMode: FileDialog.OpenFiles
        nameFilters: [i18n("Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif)"), i18n("All files (*)")]
        onAccepted: {
            const urls = []
            for (const f of fileDialog.files)
                urls.push(f.toString())
            if (urls.length)
                root.uris = urls
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 30
        anchors.rightMargin: 30
        anchors.topMargin: 32
        anchors.bottomMargin: 24
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 200
            Layout.topMargin: 0
            border.color: "#484848"
            border.width: 1
            clip: true
            color: "transparent"
            radius: 12

            PlasmaComponents.Label {
                anchors.centerIn: parent
                text: i18n("Drop or click\nto add image")
                color: "#ffffff"
                font.pixelSize: 16
                font.weight: Font.Normal
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignTop
            }

            MouseArea {
                anchors.fill: parent
                onClicked: fileDialog.open()
            }

            DropArea {
                anchors.fill: parent
                onDropped: (drop) => {
                    drop.accepted = true
                    root.uris = drop.urls.filter(u => u.toString().startsWith("file://"))
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            Layout.topMargin: 26
            spacing: 20

            PlasmaComponents.Label {
                Layout.fillWidth: true
                text: i18n("Quality")
                color: "#ffffff"
                font.pixelSize: 16
                font.weight: Font.Medium
            }
            PlasmaComponents.SpinBox {
                id: qualitySpin
                Layout.preferredWidth: 110
                Layout.preferredHeight: 32
                from: 1
                to: 100
                value: root.quality
                editable: true
                font.pixelSize: 16
                font.weight: Font.Medium
                // leftPadding handled by template; don't override
                onValueModified: root.quality = value

                background: Rectangle {
                    color: "#323232"
                    radius: 8
                    border.color: "#484848"
                    border.width: 1
                }
                contentItem: TextInput {
                    text: qualitySpin.value
                    color: "#ffffff"
                    font: qualitySpin.font
                    horizontalAlignment: Text.AlignLeft
                    verticalAlignment: Text.AlignVCenter
                    validator: IntValidator { bottom: qualitySpin.from; top: qualitySpin.to }
                    onAccepted: qualitySpin.value = Number(text)
                }
                up.indicator: Item {
                    implicitWidth: 32
                    implicitHeight: 32
                    Text {
                        anchors.centerIn: parent
                        text: "\u25B2"
                        color: "#ffffff"
                        font.pixelSize: 10
                    }
                }
                down.indicator: Item {
                    implicitWidth: 32
                    implicitHeight: 32
                    Text {
                        anchors.centerIn: parent
                        text: "\u25BC"
                        color: "#ffffff"
                        font.pixelSize: 10
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            Layout.topMargin: 22
            spacing: 20

            PlasmaComponents.Label {
                Layout.fillWidth: true
                text: i18n("Format")
                color: "#ffffff"
                font.pixelSize: 16
                font.weight: Font.Medium
            }
            PlasmaComponents.ComboBox {
                id: formatCombo
                Layout.preferredWidth: 110
                Layout.preferredHeight: 32
                font.pixelSize: 16
                font.weight: Font.Medium
                leftPadding: 13
                model: [i18n("keep"), "jpg", "png", "webp"]
                onActivated: (index) => root.format = index === 0 ? "" : model[index]

                background: Rectangle {
                    color: "#323232"
                    radius: 8
                    border.color: "#484848"
                    border.width: 1
                }
                contentItem: Text {
                    text: formatCombo.displayText
                    color: "#ffffff"
                    font: formatCombo.font
                    horizontalAlignment: Text.AlignLeft
                    verticalAlignment: Text.AlignVCenter
                }
                indicator: Text {
                    text: "\u25BC"
                    color: "#ffffff"
                    font.pixelSize: 9
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 44
            Layout.topMargin: 22
            spacing: 20

            PlasmaComponents.Button {
                id: compressButton
                Layout.preferredWidth: 120
                Layout.preferredHeight: 44
                enabled: root.uris.length > 0 && !root.busy
                onClicked: root.compressRequested()

                background: Rectangle {
                    color: parent.down ? "#3d3d3d" : (parent.hovered ? "#383838" : "#343434")
                    radius: 12
                    border.color: "#484848"
                    border.width: 1
                }
                // idle: label; busy: dots spinner + batch count
                contentItem: Item {
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        visible: !root.busy
                        text: i18n("Compress")
                        color: "#ffffff"
                        font.pixelSize: 16
                        font.weight: Font.Medium
                    }
                    Spinner {
                        anchors.centerIn: parent
                        visible: root.busy
                        running: root.busy
                        radius: 10
                    }
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        visible: root.busy && root.progressTotal > 0
                        text: i18n("%1/%2", root.progressIndex + 1, root.progressTotal)
                        color: "#ffffff"
                        font.pixelSize: 8
                    }
                }
            }

            PlasmaComponents.Button {
                Layout.preferredWidth: 120
                Layout.preferredHeight: 44
                enabled: true
                onClicked: if (root.busy) root.cancelRequested()

                background: Rectangle {
                    color: root.busy
                        ? (parent.down ? "#3d3d3d" : (parent.hovered ? "#383838" : "#343434"))
                        : "#343434"
                    radius: 12
                    border.color: root.busy ? "#484848" : "#484848"
                    border.width: 1
                }
                contentItem: Item {
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        text: i18n("Cancel")
                        color: root.busy ? "#ffffff" : "#7a7a7a"
                        font.pixelSize: 16
                        font.weight: Font.Medium
                    }
                }
            }
        }
    }
}