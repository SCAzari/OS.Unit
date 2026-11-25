// Experiment (UNIT-OS-WP-0010): the same accessible slider and text field as the spike screens,
// but built from Qt Quick Controls' standard Slider and TextField. Does Qt's AT-SPI bridge still
// crash when a reader reads them? Not one of the design's screens.
import QtQuick
import QtQuick.Controls.Basic

Rectangle {
    color: Theme.background
    Accessible.role: Accessible.Pane
    Accessible.name: "Settings"
    Column {
        x: 20; y: 60; width: parent.width - 40; spacing: 20
        Label { text: "Standard controls (experiment)"; font.pixelSize: Theme.heading }
        TextField {
            width: parent.width
            placeholderText: "Search settings (test field)"
            Accessible.name: "Search settings (test field)"
        }
        Slider {
            width: parent.width
            from: 0; to: 100; value: 45
            Accessible.name: "Glass"
        }
    }
}
