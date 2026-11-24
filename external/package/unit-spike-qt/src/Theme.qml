// Visual Design Specification r1.5 §3 tokens (Daylight, light and dark).
pragma Singleton
import QtQuick

QtObject {
    property bool dark: false
    property int accentIndex: 0
    property int themeIndex: 0
    property int shapeIndex: 1
    property real radius: 20
    property real glass: 45
    readonly property real glassLimit: 78
    readonly property var accents: ["#11756B", "#3D6FD8", "#7A5CF0", "#C2416F", "#D2512F", "#B7791F", "#2E7D4F"]
    readonly property var accentNames: ["Teal", "Blue", "Violet", "Pink", "Orange", "Amber", "Green"]
    readonly property color accent: dark && accentIndex === 0 ? "#4FC1B3" : accents[accentIndex]
    readonly property color background: dark ? "#121616" : "#F4F2EE"
    readonly property color surface: dark ? "#1B2121" : "#FFFFFF"
    readonly property color surface2: dark ? "#232A2A" : "#F0EEEA"
    readonly property color text: dark ? "#ECEFEE" : "#1E2626"
    readonly property color secondary: dark ? "#A3AEAD" : "#566362"
    readonly property color accentSoft: dark ? Qt.rgba(79 / 255, 193 / 255, 179 / 255, 0.16) : "#E3F0EE"
    readonly property color textOnAccent: dark ? "#0B1514" : "#FFFFFF"
    readonly property color glassTint: dark ? Qt.rgba(22 / 255, 28 / 255, 28 / 255, 0.72)
                                            : Qt.rgba(250 / 255, 248 / 255, 244 / 255, 0.66)
    readonly property color glassEdge: dark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(1, 1, 1, 0.55)
    readonly property string fontFamily: "Inter"
    // Type scale in points at 100% text size.
    readonly property real display: 56
    readonly property real title: 28
    readonly property real heading: 20
    readonly property real body: 16
    readonly property real label: 14
    readonly property real caption: 12.5
}
