import QtQuick

Item {
    IdleView {
        id: idle
        anchors.fill: parent
        onCompressRequested: {
            console.log("compress", idle.uris.length, "files")
        }
    }
}