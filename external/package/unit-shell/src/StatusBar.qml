// Status bar (Interface Design §5: status bar at top): the time on the left; Bluetooth, Wi-Fi
// and the battery on the right (simulated in emulation, gap G6). A layer surface on the top
// layer that keeps its height free of application windows.
import QtQuick
import QtQuick.Window
import org.kde.layershell as LayerShell
import Unit.Services

Window {
    id: bar
    readonly property int barHeight: 36
    height: barHeight
    width: 412
    visible: true
    color: "transparent"
    title: qsTr("Status bar")
    LayerShell.Window.scope: "unit-status-bar"
    LayerShell.Window.layer: LayerShell.Window.LayerTop
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.exclusionZone: barHeight
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone

    property date now: new Date()
    Timer {
        // Wake on the minute, so the time is never a minute behind.
        interval: 60000 - (bar.now.getSeconds() * 1000 + bar.now.getMilliseconds()) + 50
        running: true
        repeat: true
        onTriggered: bar.now = new Date()
    }

    Text {
        x: 18
        anchors.verticalCenter: parent.verticalCenter
        text: Qt.formatTime(bar.now, "hh:mm")
        color: Theme.secondary
        font.family: Theme.fontFamily
        font.pixelSize: 14
        font.weight: Font.Medium
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }
    Row {
        anchors.right: parent.right
        anchors.rightMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10
        Glyph {
            visible: Simulated.bluetooth
            name: "bluetooth"; color: Theme.secondary; width: 18; height: 18
            anchors.verticalCenter: parent.verticalCenter
            Accessible.ignored: false
            Accessible.role: Accessible.StaticText
            Accessible.name: qsTr("Bluetooth on")
        }
        Glyph {
            name: Simulated.wifi ? "wifi" : "wifi-off"; color: Theme.secondary; width: 18; height: 18
            anchors.verticalCenter: parent.verticalCenter
            Accessible.ignored: false
            Accessible.role: Accessible.StaticText
            Accessible.name: Simulated.wifi ? qsTr("Wi-Fi connected") : qsTr("Wi-Fi off")
        }
        Text {
            text: Simulated.batteryLevel + "%"
            color: Theme.secondary
            font.family: Theme.fontFamily
            font.pixelSize: 14
            font.weight: Font.Medium
            anchors.verticalCenter: parent.verticalCenter
            Accessible.ignored: true
        }
        // Battery outline with its level, then the terminal.
        Item {
            width: 22; height: 12
            anchors.verticalCenter: parent.verticalCenter
            Accessible.role: Accessible.StaticText
            Accessible.name: qsTr("Battery %1 percent").arg(Simulated.batteryLevel)
            Rectangle {
                width: 19; height: 12; radius: 3
                color: "transparent"
                border.width: 1.5; border.color: Theme.secondary
                Rectangle {
                    x: 2.5; y: 2.5; height: parent.height - 5; radius: 1.5
                    width: (parent.width - 5) * Simulated.batteryLevel / 100
                    color: Theme.secondary
                }
            }
            Rectangle { x: 20; y: 4; width: 2; height: 4; radius: 1; color: Theme.secondary }
        }
    }
}
