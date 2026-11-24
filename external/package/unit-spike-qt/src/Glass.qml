// VDS §3.2 glass: the backdrop blurred (36 px) and saturated, under a tint and a hairline edge.
import QtQuick
import QtQuick.Effects

Item {
    id: glass
    property Item backdrop
    property real radius: Theme.radius
    ShaderEffectSource {
        id: snap
        anchors.fill: parent
        sourceItem: glass.backdrop
        sourceRect: glass.backdrop ? glass.mapToItem(glass.backdrop, 0, 0, glass.width, glass.height)
                                   : Qt.rect(0, 0, 0, 0)
        visible: false
        live: true
    }
    Rectangle {
        id: mask
        anchors.fill: parent
        radius: glass.radius
        visible: false
        layer.enabled: true
    }
    MultiEffect {
        anchors.fill: parent
        source: snap
        blurEnabled: true
        blurMax: 36
        blur: 1.0
        saturation: 0.6
        maskEnabled: true
        maskSource: mask
    }
    Rectangle {
        anchors.fill: parent
        radius: glass.radius
        color: Theme.glassTint
        border.width: 1
        border.color: Theme.glassEdge
    }
}
