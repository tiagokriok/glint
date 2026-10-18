import QtQuick
import Quickshell
import "." as Glint
import "ui"

// Runner stages this next to Widget.qml. No observer, camera, or desktop window.
ShellRoot {
  id: test
  EdgeLightState { id: session }
  QtObject { id: service; property var lightState: session }
  QtObject {
    id: facade
    property var shell: QtObject {
      function serviceFor(id) { return id === "glint.privacy" ? service : null }
    }
    property var activePopout: null
    property string position: "top"
    property int barSize: 26
    property bool vertical: false
    function requestPopout(owner) { activePopout = owner }
    function releasePopout(owner) { if (activePopout === owner) activePopout = null }
    function showTooltip(target, text) {}
    function hideTooltip(target) {}
  }
  FloatingWindow {
    visible: true
    implicitWidth: 400; implicitHeight: 300
    Glint.Widget { id: widget; bar: facade; width: 22; height: 26 }
  }
  property var popup: null
  property real smallHeight: 0
  property real smallRegionHeight: 0
  property int phase: 0
  function child(item, name) {
    if (item.objectName === name) return item
    const children = item.data || item.children || []
    for (let i = 0; i < children.length; i++) {
      const found = child(children[i], name)
      if (found) return found
    }
    return null
  }
  function active(rows) {
    session.consumeStatusLine(JSON.stringify({version: 1, camera: {state: "active", evidence: "fixture",
      limitations: [], details: {observations: rows, truncated: rows.length > 1}}}))
  }
  function observation(index) {
    return {application: "Long observed application " + index + " x".repeat(20), applicationSource: "process",
      device: "Observed camera " + index, devicePath: "/dev/video" + index, evidence: "device-access"}
  }

  function check(condition, message) {
    if (!condition) console.error("FAIL: " + message)
    return condition
  }
  function run() {
    active([])
    session.consumeLockStatus('{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":""}')
    if (!check(widget.controlsAllowed, "fixture must allow controls")) { Qt.exit(1); return }
    // Omarchy's modulePointer forwards clicks to activeItem.triggerPress(button).
    if (!check(typeof widget.triggerPress === "function", "bar cannot forward clicks to Glint")) { Qt.exit(1); return }
    widget.triggerPress(Qt.RightButton)
    if (!check(!widget.popupOpen, "right click must not open controls")) { Qt.exit(1); return }
    widget.triggerPress(Qt.LeftButton)
    if (!check(widget.popupOpen, "forwarded left click must open controls")) { Qt.exit(1); return }
    if (!check(facade.activePopout === widget, "popup coordinator must retain the widget owner")) { Qt.exit(1); return }
    test.popup = child(widget, "edgeLightPopup")
    if (!check(test.popup !== null, "real popup must be discoverable")) { Qt.exit(1); return }
    session.protect(widget, "fixture", "top", 26)
    test.phase = 1
  }
  Timer {
    interval: 200; repeat: true; running: test.phase > 0
    onTriggered: test.advance()
  }
  function advance() {
    if (phase === 1) {
      if (!check(popup.contentWidth === popup.fittedContentWidth(360), "popup width must fit host screen")) { Qt.exit(1); return }
      smallHeight = popup.contentHeight
      const region = session.protectedRegions.find(function(r) { return r.owner === widget })
      if (!check(region && region.popup, "open popup must publish actual geometry")) { Qt.exit(1); return }
      smallRegionHeight = region.popup.height
      active(Array.from({length: 12}, function(_, index) { return observation(index) }))
      phase = 2
      return
    }
    if (phase === 2) {
      const region = session.protectedRegions.find(function(r) { return r.owner === widget })
      if (!check(popup.contentHeight > smallHeight && popup.contentHeight <= 560
          && (popup.availableCardHeight <= 0 || popup.contentHeight <= popup.availableCardHeight),
          "multiple rows must grow the bounded popup")) { Qt.exit(1); return }
      if (!check(region.popup.height > smallRegionHeight, "protected region must follow actual growth")) { Qt.exit(1); return }
      const activity = child(popup, "cameraActivity")
      if (!check(activity && activity.height <= 160 && activity.contentHeight > activity.height,
          "long activity must scroll within 160 pixels")) { Qt.exit(1); return }
      popup.contentHeight = 180
      phase = 3
      return
    }
    if (phase === 3) {
      const toggle = child(popup, "edgeLightToggle")
      const controls = child(popup, "edgeLightControlsScroll")
      const region = session.protectedRegions.find(function(r) { return r.owner === widget })
      if (!check(toggle && toggle.visible && toggle.height > 0 && toggle.y >= 0
          && toggle.y + toggle.height <= toggle.parent.height && controls.height > 0
          && controls.y + controls.height <= controls.parent.height + 1,
          "short panel must retain visible toggle and a reachable controls viewport")) { Qt.exit(1); return }
      if (!check(controls.contentHeight > controls.height, "short panel sliders must scroll")) { Qt.exit(1); return }
      controls.contentItem.contentY = controls.contentHeight - controls.height
      if (!check(region.popup.height < smallRegionHeight, "protected region must follow short panel")) { Qt.exit(1); return }
      phase = 4
      return
    }
    if (!check(child(popup, "edgeLightControlsScroll").contentItem.atYEnd,
        "last slider must be reachable by scrolling")) { Qt.exit(1); return }
    widget.triggerPress(Qt.LeftButton)
    if (!check(!widget.popupOpen, "second click must close controls")) { Qt.exit(1); return }
    session.lockFailure()
    widget.triggerPress(Qt.LeftButton)
    if (!check(!widget.popupOpen, "unknown lock state must block controls")) { Qt.exit(1); return }
    session.consumeLockStatus('{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":""}')
    widget.triggerPress(Qt.LeftButton)
    session.enableForOutput("fixture")
    session.consumeStatusLine('{"version":1,"camera":{"state":"unobserved","evidence":"stopped","limitations":[]}}')
    if (!check(!widget.popupOpen && !session.enabled && session.cameraObservations.length === 0,
        "camera stop must close panel, clear metadata and disable light")) { Qt.exit(1); return }
    if (!check(!session.protectedRegions.find(function(r) { return r.owner === widget }).popupOpen,
        "camera stop must release popup protection")) { Qt.exit(1); return }
    console.log("PASS: native triggerPress, metadata growth, protected sizing, short-panel scroll, lock and camera stop")
    Qt.exit(0)
  }
  Component.onCompleted: Qt.callLater(run)
}
