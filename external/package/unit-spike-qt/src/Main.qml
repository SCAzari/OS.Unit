// Toolkit spike window: the screen named on the command line (sign-in or settings).
import QtQuick

Window {
    id: window
    visible: true
    width: 412; height: 893
    color: Theme.background
    title: loader.item ? loader.item.Accessible.name : "Unit spike"
    property string page: startScreen === "settings" || startScreen === "display" ? startScreen : "sign-in"
    Loader {
        id: loader
        anchors.fill: parent
        focus: true
        sourceComponent: window.page === "sign-in" ? signIn : (window.page === "display" ? display : settings)
    }
    Component { id: signIn; SignIn {} }
    // The page is changed after the handler that asked for it returns (the old page is destroyed).
    Component { id: settings; SettingsHome { onOpenDisplay: Qt.callLater(() => window.page = "display") } }
    Component { id: display; DisplayAppearance { onBack: Qt.callLater(() => window.page = "settings") } }
}
