// Unit's own line icons, drawn from SVG paths in a 24-point box and coloured from the theme.
// Glyphs on glass use the full text colour (VDS §4.3).
import QtQuick
import Unit.Services

Image {
    id: glyph
    property string name
    property color color: Theme.text
    property real stroke: 1.8
    readonly property var paths: ({
        "back": "<path d='M14.5 5.5 8 12l6.5 6.5'/>",
        "home": "<circle cx='12' cy='12' r='6.5'/>",
        "switch": "<rect x='5.5' y='5.5' width='13' height='13' rx='2.5'/>",
        "wifi": "<path d='M3.5 9.5a12 12 0 0 1 17 0'/><path d='M6.5 12.8a7.6 7.6 0 0 1 11 0'/>"
              + "<path d='M9.5 16a3.2 3.2 0 0 1 5 0'/><circle cx='12' cy='19' r='0.9' fill='COLOR'/>",
        "wifi-off": "<path d='M3.5 9.5a12 12 0 0 1 17 0'/><path d='M6.5 12.8a7.6 7.6 0 0 1 11 0'/>"
                  + "<path d='M4 4l16 16'/>",
        "bluetooth": "<path d='M7 7.5l10 9-5 4.5V3l5 4.5-10 9'/>"
    })
    sourceSize.width: width
    sourceSize.height: height
    width: 24
    height: 24
    fillMode: Image.PreserveAspectFit
    smooth: true
    source: "data:image/svg+xml;base64," + Qt.btoa(
        ("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='COLOR' "
         + "stroke-width='" + stroke + "' stroke-linecap='round' stroke-linejoin='round'>"
         + (paths[name] || "") + "</svg>").replace(/COLOR/g, color.toString()))
    Accessible.ignored: true
}
