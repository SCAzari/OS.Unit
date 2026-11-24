// One choice in a group (theme card, mode segment or icon shape): a radio button for assistive
// technology, drawn by the group's delegate.
import QtQuick

Rectangle {
    id: choice
    property string label
    property bool checked: false
    property Item nextItem
    property Item prevItem
    property bool showLabel: true
    property color labelColor: Theme.text
    signal activated()
    radius: Theme.radius * 0.75
    activeFocusOnTab: true
    KeyNavigation.tab: nextItem
    KeyNavigation.backtab: prevItem
    Keys.onSpacePressed: activated()
    Keys.onReturnPressed: activated()
    Accessible.role: Accessible.RadioButton
    Accessible.name: label
    Accessible.checkable: true
    Accessible.checked: checked
    Accessible.focusable: true
    Accessible.onPressAction: activated()
    Accessible.onToggleAction: activated()
    Text {
        visible: choice.showLabel
        anchors.left: parent.left; anchors.leftMargin: 12
        anchors.bottom: parent.bottom; anchors.bottomMargin: 10
        text: choice.label; color: choice.labelColor
        font.family: Theme.fontFamily; font.pixelSize: Theme.body; font.weight: Font.DemiBold
        Accessible.ignored: true
    }
    TapHandler { onTapped: choice.activated() }
    FocusRing { controlRadius: choice.radius }
}
