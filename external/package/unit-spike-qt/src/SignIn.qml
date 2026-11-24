// SC13 Sign in, used as the lock screen (issue #8 answer 8a), after mock-up P01.
// Declared switch-scanning order (spike/screens.json): Accessibility, keypad 1-9, 0, Delete
// digit, Switch profile, Forgot PIN?
import QtQuick

Item {
    id: root
    property int digits: 0
    focus: true
    Keys.onTabPressed: a11y.forceActiveFocus(Qt.TabFocusReason)
    Accessible.role: Accessible.Pane
    Accessible.name: "Sign in"

    Rectangle {
        id: wallpaper
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: Theme.dark ? "#16201F" : "#EFEDE8" }
            GradientStop { position: 0.55; color: Theme.dark ? "#121A1A" : "#ECEBE6" }
            GradientStop { position: 1.0; color: Theme.dark ? "#0F1716" : "#E3ECEA" }
        }
        Rectangle { x: 260; y: 520; width: 220; height: 220; radius: 110; color: Theme.accentSoft; opacity: 0.8 }
        Rectangle { x: -60; y: 180; width: 200; height: 200; radius: 100; color: Theme.accentSoft; opacity: 0.6 }
    }
    StatusBar { id: status; anchors.top: parent.top; width: parent.width }
    Text {
        id: clock
        anchors.top: status.bottom; anchors.topMargin: 36
        anchors.horizontalCenter: parent.horizontalCenter
        text: "09:41"; color: Theme.text
        font.family: Theme.fontFamily; font.pixelSize: 72; font.weight: Font.Bold
        Accessible.role: Accessible.StaticText; Accessible.name: "Time, 09:41"
    }
    Text {
        id: date
        anchors.top: clock.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        text: "Saturday 14 November"; color: Theme.secondary
        font.family: Theme.fontFamily; font.pixelSize: Theme.heading
        Accessible.role: Accessible.StaticText; Accessible.name: text
    }
    Glass {
        id: note
        backdrop: wallpaper
        anchors.top: date.bottom; anchors.topMargin: 40
        x: 20; width: parent.width - 40; height: 64
        Rectangle {
            x: 16; anchors.verticalCenter: parent.verticalCenter
            width: 34; height: 34; radius: 10; color: Theme.accentSoft
            Text { anchors.centerIn: parent; text: "〒"; color: Theme.accent; font.pixelSize: 18; Accessible.ignored: true }
        }
        Column {
            x: 64; anchors.verticalCenter: parent.verticalCenter
            Text { text: "1 new message"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.body; font.weight: Font.DemiBold; Accessible.ignored: true }
            Text { text: "Sign in to see details"; color: Theme.text; font.family: Theme.fontFamily; font.pixelSize: Theme.label; Accessible.ignored: true }
        }
        Accessible.role: Accessible.StaticText
        Accessible.name: "1 new message. Sign in to see details"
    }
    Glass {
        id: card
        backdrop: wallpaper
        anchors.top: note.bottom; anchors.topMargin: 20
        // Mock-up P01: keypad rows on a 66-point pitch, buttons 24 points above the card's bottom.
        x: 20; width: parent.width - 40; height: 432
        Rectangle {
            id: avatar
            x: 24; y: 24; width: 48; height: 48; radius: 24; color: Theme.accent
            Text { anchors.centerIn: parent; text: "P"; color: Theme.textOnAccent; font.family: Theme.fontFamily; font.pixelSize: 22; font.weight: Font.DemiBold; Accessible.ignored: true }
        }
        Text {
            id: profile
            anchors.left: avatar.right; anchors.leftMargin: 14; y: 24
            text: "Profile 1"; color: Theme.text
            font.family: Theme.fontFamily; font.pixelSize: Theme.heading; font.weight: Font.DemiBold
            Accessible.role: Accessible.StaticText; Accessible.name: text
        }
        Text {
            anchors.left: profile.left; anchors.top: profile.bottom
            text: "Enter your PIN"; color: Theme.text
            font.family: Theme.fontFamily; font.pixelSize: Theme.label
            Accessible.role: Accessible.StaticText; Accessible.name: text
        }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 24; y: 40
            spacing: 8
            Accessible.role: Accessible.StaticText
            Accessible.name: "PIN, " + root.digits + " of 6 digits entered"
            Repeater {
                model: 6
                Rectangle {
                    width: 12; height: 12; radius: 6
                    color: index < root.digits ? Theme.text : "transparent"
                    border.width: 1.5; border.color: Theme.text
                    Accessible.ignored: true
                }
            }
        }
        Grid {
            id: pad
            x: 24; y: 92
            columns: 3; columnSpacing: 12; rowSpacing: 10
            KeyButton { id: k1; label: "1"; width: 100; prevItem: a11y; nextItem: k2; onActivated: root.addDigit() }
            KeyButton { id: k2; label: "2"; width: 100; prevItem: k1; nextItem: k3; onActivated: root.addDigit() }
            KeyButton { id: k3; label: "3"; width: 100; prevItem: k2; nextItem: k4; onActivated: root.addDigit() }
            KeyButton { id: k4; label: "4"; width: 100; prevItem: k3; nextItem: k5; onActivated: root.addDigit() }
            KeyButton { id: k5; label: "5"; width: 100; prevItem: k4; nextItem: k6; onActivated: root.addDigit() }
            KeyButton { id: k6; label: "6"; width: 100; prevItem: k5; nextItem: k7; onActivated: root.addDigit() }
            KeyButton { id: k7; label: "7"; width: 100; prevItem: k6; nextItem: k8; onActivated: root.addDigit() }
            KeyButton { id: k8; label: "8"; width: 100; prevItem: k7; nextItem: k9; onActivated: root.addDigit() }
            KeyButton { id: k9; label: "9"; width: 100; prevItem: k8; nextItem: k0; onActivated: root.addDigit() }
            Item { width: 100; height: 56 }
            KeyButton { id: k0; label: "0"; width: 100; prevItem: k9; nextItem: del; onActivated: root.addDigit() }
            KeyButton { id: del; label: "⌫"; accessibleName: "Delete digit"; width: 100; prevItem: k0; nextItem: switchProfile
                        onActivated: root.digits = Math.max(0, root.digits - 1) }
        }
        PillButton { id: forgot; label: "Forgot PIN?"; x: 24; anchors.bottom: parent.bottom; anchors.bottomMargin: 24
                     prevItem: switchProfile; nextItem: a11y }
        PillButton { id: switchProfile; label: "⇄  Switch"; accessibleName: "Switch profile"
                     anchors.right: parent.right; anchors.rightMargin: 24; anchors.bottom: parent.bottom; anchors.bottomMargin: 24
                     prevItem: del; nextItem: forgot }
    }
    Rectangle {
        id: a11y
        property Item nextItem: k1
        property Item prevItem: forgot
        x: 20; anchors.bottom: parent.bottom; anchors.bottomMargin: 24
        width: 56; height: 56; radius: 28
        color: a11yTap.pressed ? Theme.accentSoft : Theme.surface
        activeFocusOnTab: true
        KeyNavigation.tab: nextItem
        KeyNavigation.backtab: prevItem
        Accessible.role: Accessible.Button
        Accessible.name: "Accessibility"
        Accessible.focusable: true
        Text { anchors.centerIn: parent; text: "◉"; color: Theme.text; font.pixelSize: 24; Accessible.ignored: true }
        TapHandler { id: a11yTap }
        FocusRing { controlRadius: 28 }
    }
    function addDigit() { root.digits = Math.min(6, root.digits + 1) }
}
