import QtQuick
import QtQuick.Controls
import "../ui"

// Standalone simulated preview: no helper, camera, host service, or layer surface.
ApplicationWindow {
  width: 960
  height: 600
  visible: true
  title: "Glint border preview — simulated"
  color: "#202020"
  HoverHandler { id: cursor }
  ProtectedLight {
    anchors.fill: parent
    pointer: cursor.point.position
    pointerValid: cursor.hovered
    protectedRects: [Qt.rect(0,0,width,26)]
  }
  Label {
    anchors.centerIn: parent
    text: "Simulated Edge Light\n6% width · 70% brightness · 30% warmth"
    color: "white"
    horizontalAlignment: Text.AlignHCenter
  }
}
