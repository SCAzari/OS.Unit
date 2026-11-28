// Navigation bar (Interface Design §2 and §5; VDS §2): a floating glass pill at the bottom centre
// with Back, Home and Switch, always in that order with the same icons. A layer surface on the
// top layer; application windows keep clear of it.
import QtQuick
import QtQuick.Window
import org.kde.layershell as LayerShell
import Unit.Services

Window {
    id: nav
    signal back()
    signal home()
    signal switchApps()
    // The pill is 60 high and floats 12 above the bottom edge: the surface is 72 high, anchored
    // to the bottom, with the pill at its top.
    readonly property int bottomGap: 12
    readonly property int pillHeight: 60
    width: 220
    height: pillHeight + bottomGap
    visible: true
    color: "transparent"
    title: qsTr("Navigation bar")
    LayerShell.Window.scope: "unit-navigation-bar"
    LayerShell.Window.layer: LayerShell.Window.LayerTop
    LayerShell.Window.anchors: LayerShell.Window.AnchorBottom
    LayerShell.Window.exclusionZone: height
    // Touch and pointer only for now, so a tap does not take keyboard focus from the application.
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone

    component BarButton: Item {
        id: button
        property string glyph
        property string label
        signal activated()
        width: 64
        height: 56
        activeFocusOnTab: true
        Keys.onSpacePressed: activated()
        Keys.onReturnPressed: activated()
        Accessible.role: Accessible.Button
        Accessible.name: label
        Accessible.focusable: true
        Accessible.onPressAction: activated()
        Rectangle {
            anchors.centerIn: parent
            width: 48; height: 48; radius: 24
            color: Theme.text
            opacity: tap.pressed ? 0.12 : 0
        }
        Glyph { anchors.centerIn: parent; name: button.glyph; width: 26; height: 26 }
        TapHandler { id: tap; onTapped: button.activated() }
        FocusRing { controlRadius: 28 }
    }

    // Glass: the tint at the opacity the user chose, within the readability limit. The blurred
    // backdrop under glass is drawn by the compositor (VDS §4.4), which does not do so yet.
    Rectangle {
        id: pill
        width: parent.width
        height: nav.pillHeight
        radius: height / 2
        color: Theme.glass
        border.width: 1
        border.color: Theme.glassEdge
    }
    Row {
        anchors.centerIn: pill
        spacing: 4
        BarButton { id: backButton; glyph: "back"; label: qsTr("Back"); onActivated: nav.back() }
        BarButton { glyph: "home"; label: qsTr("Home"); onActivated: nav.home() }
        BarButton { glyph: "switch"; label: qsTr("Open applications"); onActivated: nav.switchApps() }
    }
}
