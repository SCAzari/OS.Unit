// PIN keypad key: 64 x 56 points (touch target at least 48), pressed state drawn at once.
import QtQuick

Rectangle {
    id: key
    property string label
    property string accessibleName: label
    property Item nextItem
    property Item prevItem
    signal activated()
    implicitWidth: 64; implicitHeight: 56
    radius: Theme.radius * 0.75
    color: tap.pressed ? Theme.accentSoft : Qt.rgba(Theme.surface.r, Theme.surface.g, Theme.surface.b, 0.7)
    border.width: 1
    border.color: Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.15)
    activeFocusOnTab: true
    KeyNavigation.tab: nextItem
    KeyNavigation.backtab: prevItem
    Keys.onSpacePressed: activated()
    Keys.onReturnPressed: activated()
    Accessible.role: Accessible.Button
    Accessible.name: accessibleName
    Accessible.focusable: true
    Accessible.onPressAction: activated()
    Text {
        anchors.centerIn: parent
        text: key.label; color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: 28; font.weight: Font.Medium
        Accessible.ignored: true
    }
    TapHandler { id: tap; onTapped: key.activated() }
    FocusRing { controlRadius: key.radius }
}
