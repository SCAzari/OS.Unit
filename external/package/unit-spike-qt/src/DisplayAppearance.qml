// Settings > Display and appearance, after mock-up P07 (issue #8 answer 8b). Changes apply at once.
// Scanning order is the layout order (spike/screens.json).
import QtQuick

Rectangle {
    id: root
    signal back()
    color: Theme.background
    focus: true
    Keys.onTabPressed: chain[0].forceActiveFocus(Qt.TabFocusReason)
    Accessible.role: Accessible.Pane
    Accessible.name: "Display and appearance"
    property var chain: []
    readonly property var themes: [["Daylight", "#E9ECE8", "#2E8A80", "#1E2626"], ["Night Shift", "#1F2340", "#7A6CF0", "#FFFFFF"],
                                    ["Pebble", "#FBF8F3", "#EE7559", "#1E2626"]]
    readonly property var shapes: ["Square", "Rounded", "Squircle", "Circle", "Teardrop"]
    readonly property var shapeRadii: [0, 10, 16, 24, 24]

    StatusBar { id: status; anchors.top: parent.top; width: parent.width }
    Text {
        id: crumb
        anchors.top: status.bottom; anchors.topMargin: 8; x: 20
        text: "Settings"; color: Theme.secondary
        font.family: Theme.fontFamily; font.pixelSize: Theme.label
        Accessible.role: Accessible.StaticText; Accessible.name: text
    }
    Text {
        id: heading
        anchors.top: crumb.bottom; x: 20
        text: "Display and appearance"; color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: Theme.title; font.weight: Font.Bold
        Accessible.role: Accessible.Heading; Accessible.name: text
    }
    Rectangle {
        id: card
        anchors.top: heading.bottom; anchors.topMargin: 16
        x: 14; width: parent.width - 28; height: body.height + 32
        radius: Theme.radius; color: Theme.surface
        Column {
            id: body
            x: 16; y: 16; width: parent.width - 32; spacing: 10  // mock-up P07 row gaps; the card then fits above the bar
            Text {
                text: "THEME"; color: Theme.secondary; font.family: Theme.fontFamily
                font.pixelSize: Theme.label; font.weight: Font.Bold; font.letterSpacing: 1
                Accessible.role: Accessible.StaticText; Accessible.name: "Theme"
            }
            Row {
                spacing: 8
                Repeater {
                    id: themeRepeater
                    model: root.themes
                    Choice {
                        width: (body.width - 16) / 3; height: 90
                        label: modelData[0]; checked: Theme.themeIndex === index
                        color: modelData[1]; labelColor: modelData[3]
                        border.width: checked ? 2 : 1
                        border.color: checked ? Theme.accent : Qt.rgba(0, 0, 0, 0.1)
                        onActivated: Theme.themeIndex = index
                        Rectangle { x: (parent.width - 40) / 2; y: 34; width: 40; height: 10; radius: 5; color: modelData[2] }
                    }
                }
            }
            Row {
                width: body.width; height: 52
                Text {
                    anchors.verticalCenter: parent.verticalCenter; width: parent.width - modes.width
                    text: "Mode"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body
                    Accessible.role: Accessible.StaticText; Accessible.name: text
                }
                Rectangle {
                    id: modes
                    width: 200; height: 48; radius: 24; color: Theme.surface2
                    Row {
                        anchors.centerIn: parent
                        Repeater {
                            id: modeRepeater
                            model: ["Light", "Dark", "Auto"]
                            Choice {
                                width: 64; height: 40; radius: 20; showLabel: false
                                label: modelData
                                checked: index === (Theme.dark ? 1 : 0)
                                color: checked ? Theme.surface : "transparent"
                                onActivated: if (index < 2) Theme.dark = (index === 1)
                                Text { anchors.centerIn: parent; text: modelData; color: parent.checked ? Theme.text : Theme.secondary
                                       font.family: Theme.fontFamily; font.pixelSize: Theme.label; font.weight: Font.DemiBold; Accessible.ignored: true }
                            }
                        }
                    }
                }
            }
            Text {
                text: "Accent colour"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body
                Accessible.role: Accessible.StaticText; Accessible.name: text
            }
            Row {
                spacing: 0
                Repeater {
                    id: swatchRepeater
                    model: Theme.accents
                    Swatch { label: Theme.accentNames[index]; swatchColor: modelData; checked: Theme.accentIndex === index
                             onActivated: Theme.accentIndex = index }
                }
            }
            Text {
                text: "Glass"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body
                Accessible.ignored: true
                Text { anchors.left: parent.right; anchors.leftMargin: body.width - parent.width - implicitWidth
                       text: "Readability limit ¦"; color: Theme.secondary; font.family: Theme.fontFamily; font.pixelSize: Theme.label
                       Accessible.ignored: true }
            }
            UnitSlider { id: glassSlider; width: body.width; label: "Glass"; from: 0; to: 100; value: Theme.glass
                         marker: Theme.glassLimit; onValueChanged: Theme.glass = value }
            Text {
                text: "Corner rounding"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body
                Accessible.ignored: true
                Text { anchors.left: parent.right; anchors.leftMargin: body.width - parent.width - implicitWidth
                       text: Math.round(Theme.radius) + " px"; color: Theme.secondary; font.family: Theme.fontFamily; font.pixelSize: Theme.label
                       Accessible.ignored: true }
            }
            UnitSlider { id: cornerSlider; width: body.width; label: "Corner rounding"; from: 0; to: 32; value: Theme.radius
                         valueText: Math.round(value) + " px"; onValueChanged: Theme.radius = value }
            Text {
                text: "Icon shape"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body
                Accessible.role: Accessible.StaticText; Accessible.name: text
            }
            Row {
                spacing: 12
                Repeater {
                    id: shapeRepeater
                    model: root.shapes
                    Choice {
                        width: 48; height: 48; showLabel: false
                        label: modelData; checked: Theme.shapeIndex === index
                        radius: root.shapeRadii[index]
                        color: Theme.accentSoft
                        border.width: checked ? 2 : 0; border.color: Theme.accent
                        onActivated: Theme.shapeIndex = index
                    }
                }
            }
            Rectangle {
                id: fontRow
                property Item nextItem
                property Item prevItem
                width: body.width; height: 52; radius: Theme.radius * 0.5
                color: fontTap.pressed ? Theme.accentSoft : "transparent"
                activeFocusOnTab: true
                KeyNavigation.tab: nextItem
                KeyNavigation.backtab: prevItem
                Accessible.role: Accessible.Button; Accessible.name: "Font, Inter"; Accessible.focusable: true
                Text { anchors.verticalCenter: parent.verticalCenter; text: "Font"; color: Theme.text
                       font.family: Theme.fontFamily; font.pixelSize: Theme.body; Accessible.ignored: true }
                Text { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; text: "Inter  ›"
                       color: Theme.secondary; font.family: Theme.fontFamily; font.pixelSize: Theme.body; Accessible.ignored: true }
                TapHandler { id: fontTap }
                FocusRing { controlRadius: fontRow.radius }
            }
        }
    }
    Text {
        anchors.top: card.bottom; anchors.topMargin: 12; x: 20; width: parent.width - 40
        wrapMode: Text.WordWrap
        text: "Changes apply at once, on the touchscreen and in VR. Reset to default returns to Daylight."
        color: Theme.secondary; font.family: Theme.fontFamily; font.pixelSize: Theme.label
        Accessible.role: Accessible.StaticText; Accessible.name: text
    }
    NavBar {
        id: nav
        anchors.bottom: parent.bottom; anchors.bottomMargin: 16
        anchors.horizontalCenter: parent.horizontalCenter
        onBackPressed: root.back()
    }
    Component.onCompleted: {
        const c = []
        for (let i = 0; i < 3; ++i) c.push(themeRepeater.itemAt(i))
        for (let i = 0; i < 3; ++i) c.push(modeRepeater.itemAt(i))
        for (let i = 0; i < 7; ++i) c.push(swatchRepeater.itemAt(i))
        c.push(glassSlider, cornerSlider)
        for (let i = 0; i < 5; ++i) c.push(shapeRepeater.itemAt(i))
        c.push(fontRow, nav.backButton, nav.homeButton, nav.switchButton)
        for (let i = 0; i < c.length; ++i) {
            c[i].nextItem = c[(i + 1) % c.length]
            c[i].prevItem = c[(i + c.length - 1) % c.length]
        }
        chain = c
    }
}
