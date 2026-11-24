// Three-button bar (Back, Home, Switch) floating at the bottom centre (Interface Design §6).
import QtQuick

Rectangle {
    id: bar
    property Item backdrop
    property alias backButton: back
    property alias homeButton: home
    property alias switchButton: sw
    signal backPressed()
    width: 220; height: 60; radius: 30
    color: Theme.surface2
    border.width: 1; border.color: Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.2)
    component BarButton: Item {
        id: b
        property string glyph
        property string name
        property Item nextItem
        property Item prevItem
        signal activated()
        width: 56; height: 56
        activeFocusOnTab: true
        KeyNavigation.tab: nextItem
        KeyNavigation.backtab: prevItem
        Keys.onSpacePressed: activated()
        Keys.onReturnPressed: activated()
        Accessible.role: Accessible.Button
        Accessible.name: name
        Accessible.focusable: true
        Accessible.onPressAction: activated()
        Text {
            anchors.centerIn: parent; text: b.glyph; color: Theme.text; font.pixelSize: 24
            Accessible.ignored: true
        }
        TapHandler { onTapped: b.activated() }
        FocusRing { controlRadius: 28 }
    }
    Row {
        anchors.centerIn: parent
        spacing: 8
        BarButton { id: back; glyph: "‹"; name: "Back"; onActivated: bar.backPressed() }
        BarButton { id: home; glyph: "○"; name: "Home" }
        BarButton { id: sw; glyph: "▢"; name: "Open applications" }
    }
}
