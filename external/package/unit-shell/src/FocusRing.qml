// Focus ring (VDS §4.3): a 3 px ring in the accent colour with a 2 px gap in the background
// colour, shown only while its control has keyboard focus.
import QtQuick
import Unit.Services

Rectangle {
    property real controlRadius: 0
    anchors.fill: parent
    anchors.margins: -5
    radius: controlRadius + 5
    color: "transparent"
    border.width: 3
    border.color: Theme.accent
    visible: parent.activeFocus
    Accessible.ignored: true
    Rectangle {
        anchors.fill: parent
        anchors.margins: 3
        radius: Math.max(0, parent.radius - 3)
        color: "transparent"
        border.width: 2
        border.color: Theme.background
    }
}
