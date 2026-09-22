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
    signal compressRequested

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

        PlasmaComponents.Button {
            Layout.fillWidth: true
            text: i18n("compress")
            enabled: root.uris.length > 0
            onClicked: root.compressRequested()
        }
    }
}