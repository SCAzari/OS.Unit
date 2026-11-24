// VDS §4.3 focus ring: 3 px accent ring with a 2 px gap in the background colour.
import QtQuick

Rectangle {
    property real controlRadius: 0
    anchors.fill: parent
    anchors.margins: -5
    radius: controlRadius + 5
    color: "transparent"
    border.width: 3
    border.color: Theme.accent
    visible: parent.activeFocus
    Rectangle {
        anchors.fill: parent
        anchors.margins: 3
        radius: parent.radius - 3
        color: "transparent"
        border.width: 2
        border.color: Theme.background
    }
}
