// Slider with an outlined track (VDS §4.3), accent fill and handle; arrow keys step it.
import QtQuick

Item {
    id: slider
    property string label
    property real from: 0
    property real to: 100
    property real value: 0
    property string valueText: Math.round(value).toString()
    property real marker: -1
    property Item nextItem
    property Item prevItem
    implicitHeight: 48
    activeFocusOnTab: true
    KeyNavigation.tab: nextItem
    KeyNavigation.backtab: prevItem
    Keys.onLeftPressed: value = Math.max(from, value - 1)
    Keys.onRightPressed: value = Math.min(to, value + 1)
    Accessible.role: Accessible.Slider
    Accessible.name: label
    Accessible.focusable: true
    Accessible.onIncreaseAction: value = Math.min(to, value + 1)
    Accessible.onDecreaseAction: value = Math.max(from, value - 1)
    // QAccessibleValueInterface for QML items comes from these properties.
    property real minimumValue: from
    property real maximumValue: to
    property real stepSize: 1
    readonly property real fraction: (value - from) / (to - from)
    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        x: 12; width: parent.width - 24; height: 6; radius: 3
        color: Theme.surface2
        border.width: 1.5; border.color: Theme.secondary
        Rectangle {
            width: track.width * slider.fraction; height: parent.height; radius: 3
            color: Theme.accent
        }
        Rectangle {
            visible: slider.marker >= 0
            x: track.width * (slider.marker - slider.from) / (slider.to - slider.from) - 1
            anchors.verticalCenter: parent.verticalCenter
            width: 2; height: 24; color: Theme.secondary
        }
    }
    Rectangle {
        x: track.x + track.width * slider.fraction - width / 2
        anchors.verticalCenter: parent.verticalCenter
        width: 26; height: 26; radius: 13
        color: Theme.surface
        border.width: 3; border.color: Theme.accent
    }
    DragHandler {
        target: null
        xAxis.enabled: true; yAxis.enabled: false
        onCentroidChanged: if (active) slider.value = Math.round(Math.max(slider.from, Math.min(slider.to,
            slider.from + (centroid.position.x - track.x) / track.width * (slider.to - slider.from))))
    }
    TapHandler {
        onTapped: (point) => slider.value = Math.round(Math.max(slider.from, Math.min(slider.to,
            slider.from + (point.position.x - track.x) / track.width * (slider.to - slider.from))))
    }
    FocusRing { controlRadius: 12 }
}
