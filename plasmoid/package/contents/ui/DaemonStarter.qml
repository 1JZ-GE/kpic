import QtQuick
import org.kde.plasma.workspace.dbus as DBus
import org.kde.plasma.plasma5support as P5Support

Item {
    id: root
    readonly property string service: "org.kpic.ImgSqueeze"
    readonly property string bin: Qt.resolvedUrl("../daemon/kpic").toString().substring(7)

    // spawn the daemon unless its bus name is already registered
    function ensure() {
        if (watcher.registered) return
        runner.connectSource(root.bin + " daemon")
    }

    Component.onCompleted: root.ensure()

    DBus.DBusServiceWatcher {
        id: watcher
        busType: DBus.BusType.Session
        watchedService: root.service
    }

    P5Support.DataSource {
        id: runner
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            if (data["exit code"] !== 0)
                console.error("kpic daemon start failed", data["stderr"])
            runner.disconnectSource(source)
        }
    }
}