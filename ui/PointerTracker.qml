import QtQuick
import Quickshell
import Quickshell.Io
import "PointerProtocol.js" as Protocol

Item {
  id: root
  property bool active: false
  property string socketPath: Quickshell.env("XDG_RUNTIME_DIR") + "/hypr/" + Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE") + "/.socket.sock"
  property bool valid: false
  property real globalX: 0
  property real globalY: 0
  property var request: null
  property int epoch: 0
  property double lastRequestAt: 0

  function cancel() {
    request = null
    valid = false
    deadline.stop()
    expiry.stop()
    socket.connected = false
  }
  function poll() {
    if (!active || request || retry.running) return
    const now = Date.now()
    if (!Protocol.canRequest(now,lastRequestAt)) return
    lastRequestAt = now
    request = Protocol.begin(now,epoch)
    socket.connected = true
    deadline.restart()
  }
  function fail() { cancel(); if (active) retry.restart() }
  function finish() {
    if (!request) return
    const result = Protocol.finish(request,Date.now(),epoch)
    request = null
    deadline.stop()
    if (active && result.valid) {
      globalX = result.x
      globalY = result.y
      valid = true
      expiry.restart()
    } else valid = false
  }
  onActiveChanged: {
    epoch++
    retry.stop()
    cancel()
    if (active) Qt.callLater(poll)
  }
  Socket {
    id: socket
    path: root.socketPath
    parser: SplitParser {
      splitMarker: ""
      onRead: function(chunk) {
        if (!root.request) return
        root.request = Protocol.append(root.request,String(chunk))
        if (root.request.failed) { root.fail(); return }
        // A complete JSON response is framed without waiting for PeerClosedError.
        try {
          JSON.parse(root.request.buffer)
          root.finish()
          socket.connected = false
        } catch (error) {}
      }
    }
    onConnectionStateChanged: {
      if (connected) {
        if (!root.active || !root.request) { root.cancel(); return }
        socket.write("j/cursorpos")
        socket.flush()
      } else if (root.request) {
        root.finish()
      }
    }
    onError: if (root.request) root.fail()
  }
  Timer { id: deadline; interval: 250; onTriggered: root.fail() }
  Timer { id: retry; interval: 250 }
  Timer { id: expiry; interval: 250; onTriggered: root.valid = false }
  Timer {
    interval: 50; repeat: true; running: root.active
    onTriggered: {
      root.poll()
    }
  }
  Component.onDestruction: cancel()
}
