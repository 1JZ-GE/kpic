import QtQuick

Item {
    id: root
    // urls dropped on the compact icon while the popup was closed
    property var initialUrls: []
    // dbus client owned by main.qml; drives busy + progress signals
    property var kpicClient: null
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
        busy: root.kpicClient?.busy ?? false
        onCompressRequested: {
            root.compressRequested(idle.uris, idle.quality, idle.lossless, idle.format)
        }
    }

    Connections {
        target: root.kpicClient
        function onProgressChanged(index, done, total, message) {
            idle.progressIndex = index
            idle.progressDone = done
            idle.progressTotal = total
        }
    }
}