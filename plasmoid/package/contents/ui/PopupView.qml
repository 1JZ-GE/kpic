import QtQuick

Item {
    id: root
    // urls dropped on the compact icon while the popup was closed
    property var initialUrls: []
    signal dropsConsumed
    signal compressRequested(var uris, int quality, bool lossless, string format)

    onInitialUrlsChanged: {
        if (root.initialUrls.length) {
            idle.uris = root.initialUrls.filter(u => u.toString().startsWith("file://"))
            root.dropsConsumed()
        }
    }

    IdleView {
        id: idle
        anchors.fill: parent
        onCompressRequested: {
            root.compressRequested(idle.uris, idle.quality, idle.lossless, idle.format)
        }
    }
}