import QtQuick
import QtQuick.Shapes
import Quickshell
import qs.Commons
import qs.Ui
import "ui"

BarWidget {
  id: root
  moduleName: "glint.privacy"
  readonly property var service: bar && bar.shell ? bar.shell.serviceFor("glint.privacy") : null
  readonly property var lightState: service ? service.lightState : null
  readonly property bool cameraActive: lightState ? lightState.cameraActive : false
  readonly property bool controlsAllowed: lightState ? lightState.controlsAllowed : false
  readonly property bool pressable: controlsAllowed
  readonly property string outputName: root.QsWindow.window && root.QsWindow.window.screen
    ? root.QsWindow.window.screen.name : ""
  property bool popupOpen: false
  onPopupOpenChanged: if (lightState) lightState.protectPopup(root,popupOpen,null)
  property bool tooltipHovered: hover.hovered
  implicitWidth: cameraActive ? 22 : 0
  implicitHeight: barSize
  visible: cameraActive

  function close() { popupOpen = false }
  function protectBar() {
    if (lightState && bar && outputName) lightState.protect(root, outputName, bar.position, bar.barSize)
  }
  onOutputNameChanged: protectBar()
  onLightStateChanged: protectBar()
  onBarChanged: protectBar()
  Component.onCompleted: protectBar()
  function triggerPress(button) {
    if (button !== Qt.LeftButton || !controlsAllowed) return
    if (bar) bar.hideTooltip(root)
    popupOpen = !popupOpen
  }
  onControlsAllowedChanged: if (!controlsAllowed) close()
  onCameraActiveChanged: if (!cameraActive) close()
  Connections {
    target: root.lightState
    function onSessionReset() { root.close() }
  }
  Connections {
    target: root.bar
    function onActivePopoutChanged() {
      if (root.bar.activePopout !== root) root.close()
    }
    function onPositionChanged() { root.protectBar() }
    function onBarSizeChanged() { root.protectBar() }
  }
  Item {
    anchors.centerIn: parent
    width: 16; height: 16
    Rectangle {
      x: 0; y: 3
      width: 10; height: 10; radius: 2
      color: Color.bar.active
    }
    Shape {
      x: 11; y: 3
      width: 5; height: 10
      ShapePath {
        fillColor: Color.bar.active
        strokeColor: "transparent"
        startX: 0; startY: 3
        PathLine { x: 5; y: 0 }
        PathLine { x: 5; y: 10 }
        PathLine { x: 0; y: 7 }
        PathLine { x: 0; y: 3 }
      }
    }
  }
  HoverHandler {
    id: hover
    onHoveredChanged: {
      if (!root.bar) return
      if (hovered && root.lightState) root.bar.showTooltip(root, root.lightState.cameraEvidence)
      else root.bar.hideTooltip(root)
    }
  }
  activeFocusOnTab: cameraActive
  Keys.onSpacePressed: triggerPress(Qt.LeftButton)
  Keys.onReturnPressed: triggerPress(Qt.LeftButton)
  Accessible.name: "Glint Edge Light"
  Accessible.description: lightState ? lightState.cameraEvidence : "Camera observation unavailable"
  Accessible.role: Accessible.Button
  Accessible.onPressAction: triggerPress(Qt.LeftButton)

  EdgeLightPopup {
    anchorItem: root
    bar: root.bar
    owner: root
    open: root.popupOpen && root.controlsAllowed
    lightState: root.lightState
    outputName: root.outputName
    onProtectedRegion: function(open, rect) {
      if (root.lightState) root.lightState.protectPopup(root, open, rect)
    }
  }
  Component.onDestruction: {
    if (lightState) lightState.unprotect(root)
    if (bar) {
      bar.hideTooltip(root)
      bar.releasePopout(root)
    }
  }
}
