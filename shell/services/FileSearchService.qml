pragma Singleton
import QtQuick
import Quickshell.Io
import "."

QtObject {
    id: root
    property string query: ""
    property var results: []
    property int generation: 0

    function search(text) {
        query = text.trim()
        generation++
        results = []
        if (worker.running) sendQuery()
        else worker.running = true
    }
    function sendQuery() {
        worker.write(JSON.stringify({ query: query, id: generation }) + "\n")
    }
    property Process worker: Process {
        command: ["python3", "-u", Paths.configDir + "/scripts/launcher-files.py"]
        running: true
        stdinEnabled: true
        onStarted: root.sendQuery()
        stdout: SplitParser {
            onRead: data => {
                try {
                    const response = JSON.parse(data)
                    if (response.id === root.generation && response.query === root.query)
                        root.results = response.results
                } catch (error) { console.warn("File search response:", error) }
            }
        }
        onExited: root.results = []
    }
}
