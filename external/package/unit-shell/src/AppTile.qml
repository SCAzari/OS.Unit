// An application on the home grid or in the dock: a solid tile in the user's icon shape with the
// application's glyph in the accent colour (VDS §4.3: tiles are solid, so accent glyphs sit on
// an opaque surface), and its name below on the grid. Until applications ship Unit icons, the
// glyph is the first letter of the name.
import QtQuick
import Unit.Services

Item {
    id: tile
    property string appId
    property string name
    property real tileSize: 62
    property bool showName: true
    signal activated()
    width: Math.max(tileSize, 88)
    height: tileSize + (showName ? 32 : 0)
    activeFocusOnTab: true
    Keys.onSpacePressed: activated()
    Keys.onReturnPressed: activated()
    Accessible.role: Accessible.Button
    Accessible.name: name
    Accessible.focusable: true
    Accessible.onPressAction: activated()

    Rectangle {
        id: face
        anchors.horizontalCenter: parent.horizontalCenter
        width: tile.tileSize
        height: tile.tileSize
        // Icon shape (VDS §3.3). A teardrop is a circle with one sharp corner; squircle uses the
        // large rounding until a true superellipse is drawn.
        radius: Theme.iconShape === "square" ? 0
              : Theme.iconShape === "circle" || Theme.iconShape === "teardrop" ? width / 2
              : Theme.iconShape === "squircle" ? Math.min(width * 0.4, Theme.radiusLarge)
              : Math.min(width / 2, Theme.radius)
        topRightRadius: Theme.iconShape === "teardrop" ? Theme.radiusSmall : radius
        color: Theme.surface
        border.width: 1
        border.color: Qt.rgba(Theme.secondary.r, Theme.secondary.g, Theme.secondary.b, 0.18)
        scale: tap.pressed ? 0.96 : 1
        Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        Text {
            anchors.centerIn: parent
            text: tile.name.length > 0 ? tile.name.charAt(0).toUpperCase() : "?"
            color: Theme.accent
            font.family: Theme.fontFamily
            font.pixelSize: tile.tileSize * 0.42
            font.weight: Font.DemiBold
            Accessible.ignored: true
        }
        FocusRing { controlRadius: face.radius }
    }
    Text {
        visible: tile.showName
        anchors.top: face.bottom
        anchors.topMargin: 8
        anchors.horizontalCenter: parent.horizontalCenter
        width: parent.width
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        text: tile.name
        color: Theme.text
        font.family: Theme.fontFamily
        font.pixelSize: 14
        Accessible.ignored: true
    }
    TapHandler { id: tap; onTapped: tile.activated() }
}
