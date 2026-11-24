// Status bar: time, Bluetooth, Wi-Fi, battery (illustrative values).
import QtQuick

Item {
    height: 36
    Text {
        x: 18; anchors.verticalCenter: parent.verticalCenter
        text: "09:41"; color: Theme.secondary
        font.family: Theme.fontFamily; font.pixelSize: Theme.label; font.weight: Font.Medium
        Accessible.role: Accessible.StaticText; Accessible.name: "09:41"
    }
    Row {
        anchors.right: parent.right; anchors.rightMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        spacing: 12
        Text {
            text: "◆"; color: Theme.secondary; font.pixelSize: Theme.label
            Accessible.role: Accessible.StaticText; Accessible.name: "Bluetooth on"
        }
        Text {
            text: "◎"; color: Theme.secondary; font.pixelSize: Theme.label
            Accessible.role: Accessible.StaticText; Accessible.name: "Wi-Fi connected"
        }
        Text {
            text: "82%"; color: Theme.secondary
            font.family: Theme.fontFamily; font.pixelSize: Theme.label; font.weight: Font.Medium
            Accessible.role: Accessible.StaticText; Accessible.name: "Battery 82 percent"
        }
    }
}
