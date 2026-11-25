// Test-only search field for the Japanese input test (issue #8 answer 8c: the SC04 wireframe has
// no search field). Marked "test field" on screen and in its accessible name.
import QtQuick

Rectangle {
    id: field
    property alias text: input.text
    property Item nextItem
    property Item prevItem
    implicitHeight: 48
    radius: height / 2
    color: Theme.surface2
    border.width: input.activeFocus ? 2 : 1.5
    border.color: input.activeFocus ? Theme.accent : Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.5)
    TextInput {
        id: input
        anchors.fill: parent
        anchors.leftMargin: 20; anchors.rightMargin: 20
        verticalAlignment: TextInput.AlignVCenter
        color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: Theme.body
        activeFocusOnTab: true
        KeyNavigation.tab: field.nextItem
        KeyNavigation.backtab: field.prevItem
        Accessible.role: Accessible.EditableText
        Accessible.name: "Search settings (test field)"
        Accessible.focusable: true
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: input.text === "" && !input.activeFocus
            text: "Search settings (test field)"; color: Theme.secondary
            font: input.font
            Accessible.ignored: true
        }
    }
    TapHandler { onTapped: input.forceActiveFocus(Qt.MouseFocusReason) }
    FocusRing { controlRadius: field.radius; visible: input.activeFocus }
}
