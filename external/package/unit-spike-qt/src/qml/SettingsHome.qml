// SC04 Settings home from the wireframe, with a test-only search field (issue #8 answer 8c).
// Declared scanning order: the rows, Back, Home, Open applications, then the test field.
import QtQuick

Rectangle {
    id: root
    signal openDisplay()
    color: Theme.background
    focus: true
    Keys.onTabPressed: rows.itemAt(0).forceActiveFocus(Qt.TabFocusReason)
    Accessible.role: Accessible.Pane
    Accessible.name: "Settings"
    readonly property var entries: [
        ["Wi-Fi and network", "administrator"], ["Bluetooth and devices", "administrator"],
        ["Sound and audio output", ""], ["Display and appearance", ""], ["Home and navigation", ""],
        ["Battery and performance", "administrator sets limits"], ["Profiles and sign-in", ""],
        ["Privacy and accessibility", ""], ["Storage, Updates, About", ""]]
    StatusBar { id: status; anchors.top: parent.top; width: parent.width }
    Text {
        id: heading
        anchors.top: status.bottom; anchors.topMargin: 12; x: 20
        text: "Settings"; color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: Theme.title; font.weight: Font.Bold
        Accessible.role: Accessible.Heading; Accessible.name: text
    }
    SearchField {
        id: search
        anchors.top: heading.bottom; anchors.topMargin: 16
        x: 20; width: parent.width - 40
    }
    Rectangle {
        id: list
        anchors.top: search.bottom; anchors.topMargin: 16
        x: 12; width: parent.width - 24; height: col.height + 8
        radius: Theme.radius; color: Theme.surface
        Column {
            id: col; y: 4; width: parent.width
            Repeater {
                id: rows
                model: root.entries
                SettingsRow {
                    title: modelData[0]; tag: modelData[1]
                    onActivated: if (index === 3) root.openDisplay()
                }
            }
        }
    }
    NavBar {
        id: nav
        anchors.bottom: parent.bottom; anchors.bottomMargin: 16
        anchors.horizontalCenter: parent.horizontalCenter
    }
    // The declared scanning order, linked once every row exists.
    Component.onCompleted: {
        const chain = []
        for (let i = 0; i < entries.length; ++i)
            chain.push(rows.itemAt(i))
        chain.push(nav.backButton, nav.homeButton, nav.switchButton, search)
        for (let i = 0; i < chain.length; ++i) {
            chain[i].nextItem = chain[(i + 1) % chain.length]
            chain[i].prevItem = chain[(i + chain.length - 1) % chain.length]
        }
    }
}
