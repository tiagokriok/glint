import QtQuick
import Quickshell
import Quickshell.Io
import "ui"

Item {
  id: root
  property alias lightState: session
  readonly property string helperPath: decodeURIComponent(
    String(Qt.resolvedUrl("helper/target/release/glint-helper")).replace(/^file:\/\//, ""))
  readonly property string shellPath: (Quickshell.env("OMARCHY_PATH") || "/usr/share/omarchy") + "/shell"
  readonly property var targetScreen: Quickshell.screens.find(function(output) {
    return output.name === session.targetOutput
  }) || null
  property string lockBuffer: ""
  property bool lockOverflow: false
  property int probeEpoch: -1
  property bool shuttingDown: false

  EdgeLightState { id: session }

  Process {
    id: observer
    command: [root.helperPath]
    running: true
    stdout: SplitParser {
      splitMarker: ""
      onRead: function(data) { session.consumeChunk(String(data)) }
    }
    onExited: {
      session.clearObserver()
      if (!root.shuttingDown) restart.start()
    }
  }
  Timer {
    id: restart
    interval: 2000
    onTriggered: if (!root.shuttingDown) observer.running = true
  }

  function probeLock() {
    if (!session.cameraActive || lockProbe.running || shuttingDown) return
    lockBuffer = ""
    lockOverflow = false
    probeEpoch = session.cameraEpoch
    lockProbe.running = true
    probeTimeout.restart()
  }
  Process {
    id: lockProbe
    // Same public IPC call as omarchy-shell, without a shell/timeout grandchild.
    command: ["qs", "ipc", "-n", "-p", root.shellPath, "call", "--", "lock", "status"]
    stdout: SplitParser {
      splitMarker: ""
      onRead: function(data) {
        if (root.lockOverflow) return
        if (root.lockBuffer.length + data.length > 16384) {
          root.lockOverflow = true
          root.lockBuffer = ""
          session.lockFailure()
          lockProbe.running = false
        } else root.lockBuffer += data
      }
    }
    onExited: function(exitCode, exitStatus) {
      probeTimeout.stop()
      if (!session.cameraActive || root.probeEpoch !== session.cameraEpoch) return
      if (exitCode !== 0 || exitStatus !== 0 || root.lockOverflow) session.lockFailure()
      else session.consumeLockStatus(root.lockBuffer.trim(), root.probeEpoch)
    }
  }
  Timer {
    id: probeTimeout
    interval: 2000
    onTriggered: {
      root.lockOverflow = true
      session.lockFailure()
      lockProbe.running = false
    }
  }
  Timer {
    interval: 1000
    repeat: true
    running: session.cameraActive
    onTriggered: root.probeLock()
  }
  Connections {
    target: session
    function onCameraActiveChanged() {
      if (session.cameraActive) Qt.callLater(root.probeLock)
      else {
        probeTimeout.stop()
        lockProbe.running = false
      }
    }
  }
  Connections {
    target: Quickshell
    function onScreensChanged() {
      if (session.enabled && !root.targetScreen) session.outputRemoved(session.targetOutput)
    }
  }

  // Singleton service owns the only overlay; widgets never create one.
  PointerTracker {
    id: pointer
    active: session.enabled && session.controlsAllowed && root.targetScreen !== null
  }
  EdgeLightOverlay {
    lightState: session
    pointerTracker: pointer
    screen: root.targetScreen
    visible: session.enabled && session.controlsAllowed && root.targetScreen !== null
  }
  Component.onDestruction: {
    shuttingDown = true
    restart.stop()
    session.clearObserver()
    observer.running = false
    lockProbe.running = false
  }
}
