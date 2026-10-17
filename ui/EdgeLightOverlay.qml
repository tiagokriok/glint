import QtQuick
import Quickshell
import Quickshell.Wayland
import "LightGeometry.js" as Geometry

PanelWindow {
  id: root
  required property EdgeLightState lightState
  required property PointerTracker pointerTracker
  readonly property var regions: lightState.protectedRegions.filter(function(r) { return root.screen && r.output === root.screen.name })
  readonly property bool geometryReady: screen !== null && regions.length > 0 && regions.every(function(r) {
    return Geometry.barRect(root.width,root.height,r.edge,r.size) !== null && (!r.popupOpen || r.popup !== null)
  })
  readonly property var clearRects: regions.reduce(function(rects,r) {
    const barRect = Geometry.barRect(root.width,root.height,r.edge,r.size)
    const popup = r.popupOpen && r.popup && root.screen ? Qt.rect(r.popup.x-root.screen.x,r.popup.y-root.screen.y,r.popup.width,r.popup.height) : null
    return rects.concat(barRect ? [barRect] : [], popup ? [popup] : [])
  },[])
  readonly property point localPointer: screen ? Qt.point(pointerTracker.globalX-screen.x,pointerTracker.globalY-screen.y) : Qt.point(-1000,-1000)
  anchors { top: true; bottom: true; left: true; right: true }
  color: "transparent"
  exclusionMode: ExclusionMode.Ignore
  WlrLayershell.namespace: "glint-edge-light"
  WlrLayershell.layer: WlrLayer.Overlay
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
  mask: Region {}

  ProtectedLight {
    anchors.fill: parent
    visible: root.geometryReady && root.pointerTracker.valid
    protectedRects: root.clearRects
    pointer: root.localPointer
    pointerValid: root.pointerTracker.valid && root.localPointer.x >= 0 && root.localPointer.y >= 0 && root.localPointer.x < root.width && root.localPointer.y < root.height
    widthPercent: root.lightState.widthPercent
    brightnessPercent: root.lightState.brightnessPercent
    warmthPercent: root.lightState.warmthPercent
  }
}
