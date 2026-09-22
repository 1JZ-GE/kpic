import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents
import org.kde.kirigami as Kirigami

Item {
    id: root
    property var uris: []
    property int quality: 80
    property bool lossless: false
    property string format: ""
    // live job state, fed by the client via popup view
    property bool busy: false
    property int progressIndex: 0
    property int progressDone: 0
    property int progressTotal: 0
    signal compressRequested

    onBusyChanged: {
        if (root.busy) {
            root.progressIndex = 0
            root.progressDone = 0
            root.progressTotal = 0
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.gridUnit
        spacing: Kirigami.Units.smallSpacing

        Rectangle {
            id: dropZone
            Layout.fillWidth: true
            Layout.preferredHeight: 160
            color: dropArea.containsDrag ? Kirigami.Theme.highlightColor : "transparent"
            border.color: Kirigami.Theme.textColor
            border.width: 1
            radius: 4

            PlasmaComponents.Label {
                anchors.centerIn: parent
                text: root.uris.length === 0
                    ? i18n("Drop images here")
                    : i18ncp("%1 file", "%1 file", "%1 files", root.uris.length)
            }

            DropArea {
                id: dropArea
                anchors.fill: parent
                onDropped: (drop) => {
                    drop.accepted = true
                    root.uris = drop.urls.filter(u => u.toString().startsWith("file://"))
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            PlasmaComponents.Label { text: i18n("quality") }
            PlasmaComponents.Slider {
                id: qualitySlider
                Layout.fillWidth: true
                from: 1
                to: 100
                value: root.quality
                onMoved: root.quality = value
            }
            PlasmaComponents.Label { text: root.quality }
        }

        RowLayout {
            Layout.fillWidth: true
            PlasmaComponents.Label { text: i18n("format") }
            PlasmaComponents.ComboBox {
                id: formatCombo
                Layout.fillWidth: true
                model: [i18n("keep"), "jpg", "png", "webp"]
                onActivated: (index) => root.format = index === 0 ? "" : model[index]
            }
        }

        PlasmaComponents.CheckBox {
            text: i18n("lossless")
            checked: root.lossless
            onToggled: root.lossless = checked
        }

        RowLayout {
            Layout.fillWidth: true
            PlasmaComponents.Button {
                id: compressButton
                Layout.fillWidth: true
                enabled: root.uris.length > 0 && !root.busy
                onClicked: root.compressRequested()

                // idle: text label; busy: the ring takes over the button
                contentItem: Item {
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        visible: !root.busy
                        text: i18n("compress")
                    }
                    Spinner {
                        anchors.centerIn: parent
                        visible: root.busy
                        running: root.busy
                        radius: Math.max(6, parent.height / 2 - 4)
                    }
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        visible: root.busy && root.progressTotal > 0
                        text: i18n("%1/%2", root.progressIndex + 1, root.progressTotal)
                        font.pixelSize: 8
                    }
                }
            }
        }
    }
}