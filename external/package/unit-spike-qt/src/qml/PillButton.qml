// Outlined pill button (VDS §5 secondary button): accent label, pill shape.
import QtQuick

Rectangle {
    id: pill
    property string label
    property string accessibleName: label
    property Item nextItem
    property Item prevItem
    signal activated()
    implicitWidth: Math.max(120, caption.implicitWidth + 40); implicitHeight: 48
    radius: height / 2
    color: tap.pressed ? Theme.accentSoft : "transparent"
    border.width: 1.5
    border.color: Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.35)
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
        id: caption
        anchors.centerIn: parent
        text: pill.label; color: Theme.accent
        font.family: Theme.fontFamily; font.pixelSize: Theme.body; font.weight: Font.DemiBold
        Accessible.ignored: true
    }
    TapHandler { id: tap; onTapped: pill.activated() }
    FocusRing { controlRadius: pill.radius }
}
