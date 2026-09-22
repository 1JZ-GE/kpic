import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Layout.preferredWidth: 320
    Layout.preferredHeight: 440
    // keep the popup alive while the file picker owns focus; outside clicks
    // still dismiss it otherwise
    property bool filePicking: false
    hideOnWindowDeactivate: !root.filePicking

    // urls forwarded from a drop on the small panel icon
    property var pendingDrops: []

    // daemon lifecycle + dbus transport (task 7)
    DaemonStarter {
        id: starter
    }
    KpicClient {
        id: client
    }

    compactRepresentation: Item {
        id: compactRoot
        Layout.preferredWidth: Kirigami.Units.iconSizes.medium
        Layout.preferredHeight: Kirigami.Units.iconSizes.medium

        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            onClicked: root.expanded = !root.expanded

            Kirigami.Icon {
                anchors.fill: parent
                source: Qt.resolvedUrl(Kirigami.Theme.colorScheme === Kirigami.Theme.Dark
                    ? "../icons/kpic-downscale-dark.svg"
                    : "../icons/kpic-downscale.svg")
            }
        }

        // accept a drop on the icon: open the popup so the drop zone
        // inside can take the files; forward the urls as a fallback
        DropArea {
            anchors.fill: parent
            onEntered: root.expanded = true
            onDropped: (drop) => {
                root.pendingDrops = drop.urls
                root.expanded = true
            }
        }
    }

    fullRepresentation: PopupView {
        kpicClient: client
        initialUrls: root.pendingDrops
        onDropsConsumed: root.pendingDrops = []
        onCompressRequested: (uris, quality, lossless, format) => {
            starter.ensure()
            client.start(uris, quality, lossless, format)
        }
        onCancelRequested: client.cancel()
        onFilePickingChanged: (picking) => root.filePicking = picking
    }
}