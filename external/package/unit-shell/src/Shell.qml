// The Shell UI's surfaces: home on the background layer, then the status bar and the navigation
// bar above application windows.
import QtQuick
import Unit.Services

QtObject {
    id: shell
    // Pinned applications have no setting yet (Settings › Home and navigation comes later). The
    // sample applications are pinned once they are installed (gap G3).
    readonly property var pinned: ["foot"]

    property Home home: Home { pinned: shell.pinned }
    property StatusBar statusBar: StatusBar {}
    property NavigationBar navigationBar: NavigationBar {
        // Back, Home and Switch act on application windows, which come with window control
        // (the next change). Until then they only report the press.
        onBack: console.info("navigation: Back")
        onHome: console.info("navigation: Home")
        onSwitchApps: console.info("navigation: Switch")
    }
    property Connections apps: Connections {
        target: Apps
        function onFailed(appId, reason) { console.warn("could not open", appId + ":", reason) }
    }
}
