// Accent colour swatch: 48-point target with a 36-point colour disc.
import QtQuick

Item {
    id: swatch
    property string label
    property color swatchColor
    property bool checked: false
    property Item nextItem
    property Item prevItem
    signal activated()
    width: 48; height: 48
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
    Rectangle {
        anchors.centerIn: parent
        width: swatch.checked ? 44 : 36; height: width; radius: width / 2
        color: swatch.checked ? "transparent" : swatch.swatchColor
        border.width: swatch.checked ? 4 : 0
        border.color: Theme.text
        Rectangle {
            visible: swatch.checked
            anchors.centerIn: parent
            width: 32; height: 32; radius: 16
            color: swatch.swatchColor
        }
    }
    TapHandler { onTapped: swatch.activated() }
    FocusRing { controlRadius: 24 }
}
