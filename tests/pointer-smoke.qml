import QtQuick
import Quickshell
import Quickshell.Io
import "ui"

ShellRoot {
  id: test
  property int requests: 0
  property bool hold: false
  property bool oversized: false
  property int phase: 0
  property int savedRequests: 0
  property double started: Date.now()
  property double phaseAt: Date.now()
  PointerTracker { id: tracker; socketPath: server.path }
  SocketServer {
    id: server
    active: true
    path: Quickshell.env("GLINT_TEST_SOCKET")
    handler: Socket {
      id: client
      parser: SplitParser {
        splitMarker: ""
        onRead: function(data) {
          if (data !== "j/cursorpos") { console.error("unexpected request"); Qt.exit(1); return }
          test.requests++
          if (test.hold) return
          client.write(test.oversized ? "x".repeat(4097) : '{"x":-1920.5,"y":42}')
          client.flush()
          client.connected = false
        }
      }
    }
  }
  Timer {
    interval: 25; repeat: true; running: true
    onTriggered: {
      const now = Date.now()
      if (now-test.started > 4000) { console.error("pointer fixture timed out"); Qt.exit(1); return }
      if (test.phase === 0 && now-test.phaseAt > 150) {
        if (test.requests !== 0) { console.error("idle tracker queried socket"); Qt.exit(1); return }
        tracker.active = true; test.phase = 1
      } else if (test.phase === 1 && tracker.valid) {
        if (tracker.globalX !== -1920.5 || tracker.globalY !== 42) { Qt.exit(1); return }
        tracker.active = false; test.savedRequests = test.requests; test.phaseAt = now; test.phase = 2
      } else if (test.phase === 2 && now-test.phaseAt > 150) {
        if (test.requests !== test.savedRequests || tracker.valid) { console.error("tracker stayed active"); Qt.exit(1); return }
        test.hold = true; tracker.active = true; test.phaseAt = now; test.phase = 3
      } else if (test.phase === 3 && now-test.phaseAt > 400) {
        if (tracker.valid) { console.error("timeout did not invalidate pointer"); Qt.exit(1); return }
        tracker.active = false; test.hold = false; test.oversized = true
        test.phaseAt = now; test.phase = 4
      } else if (test.phase === 4 && now-test.phaseAt > 100) {
        tracker.active = true; test.phaseAt = now; test.phase = 5
      } else if (test.phase === 5 && now-test.phaseAt > 200) {
        if (tracker.valid || tracker.request && tracker.request.buffer.length > 4096) { Qt.exit(1); return }
        tracker.active = false
        console.log("PASS: native pointer socket, negative coordinates, idle, timeout and size bound")
        Qt.exit(0)
      }
    }
  }
}
