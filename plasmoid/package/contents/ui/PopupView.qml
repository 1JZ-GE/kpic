import QtQuick
import QtQuick.Layouts

Item {
    id: root
    // fixed popup size: layout min==max==preferred; shell clamps the popup
    // window to it, so the user cannot resize it
    Layout.minimumWidth: 320
    Layout.maximumWidth: 320
    Layout.preferredWidth: 320
    Layout.minimumHeight: 480
    Layout.maximumHeight: 480
    Layout.preferredHeight: 480
    // urls dropped on the compact icon while the popup was closed
    property var initialUrls: []
    // dbus client owned by main.qml; drives busy + progress signals
    property var kpicClient: null
    signal dropsConsumed
    signal compressRequested(var uris, int quality, bool lossless, string format)
    signal cancelRequested
    signal filePickingChanged(bool picking)

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
        onCancelRequested: root.cancelRequested()
        onFilePickingChanged: (picking) => root.filePickingChanged(picking)
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
