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
    property bool lossless: true
    property string format: ""
    // live job state, fed by the client via popup view
    property bool busy: false
    property int progressIndex: 0
    property int progressDone: 0
    property int progressTotal: 0
    signal compressRequested
    signal cancelRequested
    // popup-lifetime toggle while the file picker owns screen focus
    signal filePickingChanged(bool picking)

    onBusyChanged: {
        if (root.busy) {
            root.progressIndex = 0
            root.progressDone = 0
            root.progressTotal = 0
        }
    }

    function _fileName(url) {
        return String(url).replace(/^file:\/\//, "").split("/").pop()
    }

    FileDialog {
        id: fileDialog
        fileMode: FileDialog.OpenFiles
        nameFilters: [i18n("Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif)"), i18n("All files (*)")]
        // the picker is a separate window; tell the applet to keep the popup
        // open while it grabs focus (hide-on-deactivate would dismiss us)
        onVisibleChanged: root.filePickingChanged(fileDialog.visible)
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
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.bottomMargin: 12
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 180
            Layout.topMargin: 0
            border.color: Qt.alpha(Kirigami.Theme.disabledTextColor, 0.5)
            border.width: 1
            clip: true
            color: "transparent"
            radius: 12

            PlasmaComponents.Label {
                anchors.centerIn: parent
                text: root.uris.length === 0
                    ? i18n("Drop or click\nto add image")
                    : (root.uris.length === 1
                        ? root._fileName(root.uris[0])
                        : i18n("%1 images selected", root.uris.length))
                color: Kirigami.Theme.disabledTextColor
                font.pixelSize: 16
                font.weight: Font.Normal
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                wrapMode: Text.Wrap
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

            // clear
            MouseArea {
                id: clearSelection
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.topMargin: 4
                anchors.rightMargin: 4
                width: 24
                height: 24
                z: 10
                visible: root.uris.length > 0
                onClicked: root.uris = []

                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    color: Kirigami.Theme.backgroundColor
                    border.color: Qt.alpha(Kirigami.Theme.disabledTextColor, 0.5)
                    border.width: 1
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        text: "\u2715"
                        color: Kirigami.Theme.textColor
                        font.pixelSize: 12
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            Layout.topMargin: 14
            spacing: 20

            PlasmaComponents.Label {
                Layout.fillWidth: true
                text: i18n("Quality")
                color: root.lossless ? Kirigami.Theme.disabledTextColor : Kirigami.Theme.textColor
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
                enabled: !root.lossless
                font.pixelSize: 16
                font.weight: Font.Medium
                onValueModified: root.quality = value
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            Layout.topMargin: 14
            spacing: 20

            PlasmaComponents.Label {
                Layout.fillWidth: true
                text: i18n("Format")
                color: Kirigami.Theme.textColor
                font.pixelSize: 16
                font.weight: Font.Medium
            }
            PlasmaComponents.ComboBox {
                id: formatCombo
                Layout.preferredWidth: 110
                Layout.preferredHeight: 32
                font.pixelSize: 16
                font.weight: Font.Medium
                model: [i18n("keep"), "jpg", "png", "webp"]
                onActivated: (index) => root.format = index === 0 ? "" : model[index]
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            Layout.topMargin: 14
            spacing: 20

            PlasmaComponents.Label {
                id: modeLabel
                Layout.fillWidth: true
                text: i18n("Mode")
                color: Kirigami.Theme.textColor
                font.pixelSize: 16
                font.weight: Font.Medium
            }
            PlasmaComponents.CheckBox {
                id: losslessCheck
                Layout.preferredHeight: 32
                checked: root.lossless
                text: i18n("Lossless")
                font.pixelSize: 16
                font.weight: Font.Medium
                onToggled: root.lossless = checked
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            Layout.topMargin: 14
            spacing: 20

            PlasmaComponents.Button {
                id: compressButton
                Layout.fillWidth: true
                Layout.preferredHeight: 40
                enabled: root.uris.length > 0 && !root.busy
                onClicked: root.compressRequested()

                // idle: label; busy: dots spinner + batch count
                contentItem: Item {
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        visible: !root.busy
                        text: i18n("Compress")
                        color: compressButton.enabled ? Kirigami.Theme.textColor : Kirigami.Theme.disabledTextColor
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
                        color: Kirigami.Theme.textColor
                        font.pixelSize: 8
                    }
                }
            }

            PlasmaComponents.Button {
                Layout.fillWidth: true
                Layout.preferredHeight: 40
                enabled: true
                onClicked: if (root.busy) root.cancelRequested()

                contentItem: Item {
                    PlasmaComponents.Label {
                        anchors.centerIn: parent
                        text: i18n("Cancel")
                        color: root.busy ? Kirigami.Theme.textColor : Kirigami.Theme.disabledTextColor
                        font.pixelSize: 16
                        font.weight: Font.Medium
                    }
                }
            }
        }
    }
}
