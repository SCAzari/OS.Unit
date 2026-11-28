// Home screen, first release (SC01; VDS Figure 3; gap decision G4, UNIT-OS-DEC-0126): the time
// and date, the installed applications in a grid of four columns, and the dock row of pinned
// applications just above the navigation bar (Interface Design §5; UNIT-OS-DEC-0072). The
// widgets of Figure 3 follow in the next change. A layer surface on the background layer, under
// every application window, reaching under the status and navigation bars.
import QtQuick
import QtQuick.Window
import org.kde.layershell as LayerShell
import Unit.Services

Window {
    id: home
    // Pinned applications, by application ID, in dock order.
    property var pinned: []
    // Room left at the bottom for the floating navigation bar (60 high, 12 from the edge).
    property int navigationSpace: 72
    width: 412
    height: 893
    visible: true
    color: Theme.background
    title: qsTr("Home")
    LayerShell.Window.scope: "unit-home"
    LayerShell.Window.layer: LayerShell.Window.LayerBackground
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom
                               | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.exclusionZone: -1
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone

    property date now: new Date()
    Timer {
        interval: 60000 - (home.now.getSeconds() * 1000 + home.now.getMilliseconds()) + 50
        running: true
        repeat: true
        onTriggered: home.now = new Date()
    }
    function greeting(hour) {
        return hour < 5 ? qsTr("Good evening") : hour < 12 ? qsTr("Good morning")
             : hour < 18 ? qsTr("Good afternoon") : qsTr("Good evening")
    }
    function isPinned(appId) { return pinned.indexOf(appId) >= 0 }

    // Time and date (VDS display type, 56).
    Column {
        id: clock
        x: 24
        y: 36 + 24
        spacing: 4
        Text {
            text: Qt.formatTime(home.now, "hh:mm")
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: 56
            font.weight: Font.DemiBold
            Accessible.role: Accessible.StaticText
            Accessible.name: qsTr("Time %1").arg(text)
        }
        Text {
            text: Qt.formatDate(home.now, "dddd d MMMM") + " · " + home.greeting(home.now.getHours())
            color: Theme.secondary
            font.family: Theme.fontFamily
            font.pixelSize: 16
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
    }

    // Installed applications that are not pinned, four to a row, above the dock.
    Grid {
        id: grid
        columns: 4
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: dock.visible ? dock.top : parent.bottom
        anchors.bottomMargin: dock.visible ? 24 : home.navigationSpace + 24
        columnSpacing: (home.width - 32 - 4 * 88) / 3
        rowSpacing: 16
        Repeater {
            model: Apps
            AppTile {
                required appId
                required name
                visible: !home.isPinned(appId)
                onActivated: Apps.open(appId)
            }
        }
    }
    Text {
        visible: Apps.count === 0
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: home.navigationSpace + 40
        text: qsTr("No applications installed")
        color: Theme.secondary
        font.family: Theme.fontFamily
        font.pixelSize: 14
    }

    // Dock row of pinned applications: glass, with solid tiles (VDS §2, §4.3).
    Rectangle {
        id: dock
        // Pinned applications that are installed (re-counted when the list changes).
        readonly property int pinnedShown: Apps.count >= 0
            ? home.pinned.filter(id => Apps.nameOf(id) !== "").length : 0
        visible: pinnedShown > 0
        x: 20
        width: home.width - 40
        height: 72
        anchors.bottom: parent.bottom
        anchors.bottomMargin: home.navigationSpace + 16
        radius: Math.min(height / 2, Theme.radiusLarge)
        color: Theme.glass
        border.width: 1
        border.color: Theme.glassEdge
        Accessible.role: Accessible.ToolBar
        Accessible.name: qsTr("Pinned applications")
        Row {
            id: dockRow
            anchors.centerIn: parent
            spacing: 24
            Repeater {
                model: Apps
                AppTile {
                    required appId
                    required name
                    tileSize: 52
                    showName: false
                    width: 52
                    visible: home.isPinned(appId)
                    onActivated: Apps.open(appId)
                }
            }
        }
    }
}
