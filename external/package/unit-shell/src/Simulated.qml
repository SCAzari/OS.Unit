// The simulated hardware states of the emulation (gap G6, UNIT-OS-DEC-0129): Wi-Fi, Bluetooth,
// battery and the other quick-settings tiles change state in the Shell UI only. Nothing real sits
// behind them, nothing stores them, and they reset when the shell restarts. The demonstration
// script and the known-limitations list say so.
pragma Singleton
import QtQuick

QtObject {
    property bool wifi: true
    property bool bluetooth: true
    property bool batterySaver: false
    property bool rotationLock: false
    property bool vrMode: false
    // Illustrative level, as in the visual design (VDS Figure 3).
    readonly property int batteryLevel: 82
}
