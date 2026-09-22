import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as P5Support

PlasmoidItem {
    id: root
    Layout.preferredWidth: 320
    Layout.preferredHeight: 640

    // spawn the bundled daemon on load; safe to re-run, daemon exits itself
    // when the bus name is already taken
    P5Support.DataSource {
        id: daemonSource
        engine: "executable"
        connectedSources: []
        Component.onCompleted: connectSource(Qt.resolvedUrl("../daemon/imgsqueeze").toString().substring(7))
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
                source: "archive-insert"
            }
        }
    }

    fullRepresentation: PopupView {}
}
