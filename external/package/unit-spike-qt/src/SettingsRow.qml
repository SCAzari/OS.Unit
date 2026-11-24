// Settings home row: title, optional administrator tag, chevron; 56 points high.
import QtQuick

Rectangle {
    id: row
    property string title
    property string tag
    property Item nextItem
    property Item prevItem
    signal activated()
    width: parent ? parent.width : 0
    height: 56
    radius: Theme.radius * 0.5
    color: tap.pressed ? Theme.accentSoft : "transparent"
    activeFocusOnTab: true
    KeyNavigation.tab: nextItem
    KeyNavigation.backtab: prevItem
    Keys.onSpacePressed: activated()
    Keys.onReturnPressed: activated()
    Accessible.role: Accessible.Button
    Accessible.name: tag === "" ? title : title + ", " + tag
    Accessible.focusable: true
    Accessible.onPressAction: activated()
    Text {
        anchors.left: parent.left; anchors.leftMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        text: row.title; color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: Theme.body
        Accessible.ignored: true
    }
    Text {
        anchors.right: chevron.left; anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        text: row.tag; color: Theme.secondary
        font.family: Theme.fontFamily; font.pixelSize: Theme.caption
        Accessible.ignored: true
    }
    Text {
        id: chevron
        anchors.right: parent.right; anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        text: "›"; color: Theme.secondary; font.pixelSize: 22
        Accessible.ignored: true
    }
    Rectangle {
        anchors.bottom: parent.bottom; anchors.left: parent.left; anchors.right: parent.right
        anchors.leftMargin: 16; height: 1
        color: Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.15)
    }
    TapHandler { id: tap; onTapped: row.activated() }
    FocusRing { controlRadius: row.radius }
}
