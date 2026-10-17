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
  Glint.Widget { id: widget; bar: facade; width: 22; height: 26 }

  function check(condition, message) {
    if (!condition) console.error("FAIL: " + message)
    return condition
  }
  function run() {
    session.consumeStatusLine('{"version":1,"camera":{"state":"active","evidence":"fixture","limitations":[]}}')
    session.consumeLockStatus('{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":""}')
    if (!check(widget.controlsAllowed, "fixture must allow controls")) { Qt.exit(1); return }
    // Omarchy's modulePointer forwards clicks to activeItem.triggerPress(button).
    if (!check(typeof widget.triggerPress === "function", "bar cannot forward clicks to Glint")) { Qt.exit(1); return }
    widget.triggerPress(Qt.RightButton)
    if (!check(!widget.popupOpen, "right click must not open controls")) { Qt.exit(1); return }
    widget.triggerPress(Qt.LeftButton)
    if (!check(widget.popupOpen, "forwarded left click must open controls")) { Qt.exit(1); return }
    if (!check(facade.activePopout === widget, "popup coordinator must retain the widget owner")) { Qt.exit(1); return }
    widget.triggerPress(Qt.LeftButton)
    if (!check(!widget.popupOpen, "second click must close controls")) { Qt.exit(1); return }
    session.lockFailure()
    widget.triggerPress(Qt.LeftButton)
    if (!check(!widget.popupOpen, "unknown lock state must block controls")) { Qt.exit(1); return }
    console.log("PASS: forwarded bar click opens/closes controls and respects lock gate")
    Qt.exit(0)
  }
  Component.onCompleted: Qt.callLater(run)
}
