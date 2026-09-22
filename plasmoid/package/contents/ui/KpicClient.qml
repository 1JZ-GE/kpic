import QtQuick
import org.kde.plasma.workspace.dbus as DBus

Item {
    id: root
    property bool busy: false
    property int jobId: -1
    property var pending: null
    readonly property string service: "org.kpic.ImgSqueeze"
    readonly property string path: "/org/kpic/ImgSqueeze"
    readonly property string iface: "org.kpic.ImgSqueeze"

    signal progressChanged(int index, int done, int total, string message)
    signal finished(var results)

    property var msg: ({
        "service": root.service,
        "path": root.path,
        "iface": root.iface,
        "member": "",
        "arguments": [],
        "signature": null
    })

    DBus.DBusServiceWatcher {
        id: watcher
        busType: DBus.BusType.Session
        watchedService: root.service
        onRegisteredChanged: {
            // flush a call that waited for the daemon to come up
            if (watcher.registered && root.pending) {
                const args = root.pending
                root.pending = null
                root.doCall("StartJob", "(asybs)", args)
            }
        }
    }

    DBus.SignalWatcher {
        id: signals
        busType: DBus.BusType.Session
        service: root.service
        path: root.path
        iface: root.iface
        function dbusProgressChanged(jobId, index, done, total, message) {
            root.progressChanged(num(index), num(done), num(total), String(message?.value ?? message ?? ""))
        }
        function dbusJobFinished(jobId, uris, outputs, inputSizes, outputSizes, errors) {
            root.finished(zipResults(uris, outputs, inputSizes, outputSizes, errors))
            root.jobId = -1
            root.busy = false
        }
    }

    function start(uris, quality, lossless, format) {
        if (root.busy) return
        if (!uris || uris.length === 0) return
        root.busy = true
        const args = [uris, quality, lossless, format]
        if (watcher.registered)
            root.doCall("StartJob", "(asybs)", args)
        else
            root.pending = args
    }

    function doCall(member, signature, args) {
        root.msg = {
            "service": root.service,
            "path": root.path,
            "iface": root.iface,
            "member": member,
            "arguments": args,
            "signature": signature
        }
        const reply = DBus.SessionBus.asyncCall(root.msg) as DBus.DBusPendingReply
        reply.finished.connect(() => {
            if (reply.isError) {
                root.busy = false
                console.error("kpic dbus call failed", reply.error?.message ?? reply.error?.name)
            } else if (reply.isValid && member === "StartJob") {
                const v = reply.value
                root.jobId = v?.value ?? v
            }
            reply.destroy()
        })
    }

    function cancel() {
        if (root.jobId < 0) return
        root.doCall("Cancel", "(u)", [root.jobId])
    }

    // scalar dbus args arrive as { value } wrappers; normalize
    function num(v) { return v && v.value !== undefined ? v.value : v }

    function zipResults(uris, outputs, ins, outs, errors) {
        const result = []
        const urisArr = uris ?? [], outputArr = outputs ?? [], insArr = ins ?? [],
              outsArr = outs ?? [], errsArr = errors ?? []
        for (let i = 0; i < urisArr.length; i++) {
            result.push({
                uri: num(urisArr[i]),
                output: num(outputArr[i] ?? ""),
                inputSize: num(insArr[i] ?? 0),
                outputSize: num(outsArr[i] ?? 0),
                error: num(errsArr[i] ?? "")
            })
        }
        return result
    }
}