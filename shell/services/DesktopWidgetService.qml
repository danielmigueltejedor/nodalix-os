pragma Singleton
import QtQuick

QtObject {
    id: root
    property bool editMode: false
    function toggleEdit() { editMode = !editMode }
    function closeEdit() { editMode = false }
}
