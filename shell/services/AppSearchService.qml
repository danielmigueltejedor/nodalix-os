pragma Singleton
import QtQuick
import QtQml.WorkerScript
import "."

QtObject {
    id: root
    property string query: ""
    property var results: []
    property bool busy: false
    property int generation: 0
    readonly property var records: AppService.searchIndex.map((r, i) => ({
        index: i, name: r.name, generic: r.generic, keywords: r.keywords
    }))
    onRecordsChanged: rebuild()
    function rebuild() {
        generation++
        results = []
        if (!worker.ready) return
        worker.sendMessage({ kind: "catalog", records: records })
        submit()
    }
    function submit() {
        busy = query.length > 0
        if (worker.ready)
            worker.sendMessage({ kind: "query", query: query, generation: generation })
    }
    function search(text) {
        const next = text.trim().toLowerCase()
        if (query === next) return
        query = next
        generation++
        results = []
        submit()
    }
    property WorkerScript worker: WorkerScript {
        source: Qt.resolvedUrl("AppSearchWorker.js")
        onReadyChanged: if (ready) root.rebuild()
        onMessage: message => {
            if (message.generation !== root.generation) return
            root.results = message.indices.map(i => AppService.apps[i]).filter(e => !!e)
            root.busy = false
        }
    }
    Component.onCompleted: rebuild()
}
