import QtQuick

QtObject {
  signal sessionReset()
  property var protectedRegions: []
  function protect(owner, output, edge, size) {
    const previous = protectedRegions.find(function(r) { return r.owner === owner })
    protectedRegions = protectedRegions.filter(function(r) { return r.owner !== owner }).concat([
      {owner: owner, output: output, edge: edge, size: size,
        popupOpen: previous ? previous.popupOpen : false, popup: previous ? previous.popup : null}])
  }
  function protectPopup(owner, open, rect) {
    const previous = protectedRegions.find(function(r) { return r.owner === owner })
    if (!previous) return
    const p = previous.popup
    if (previous.popupOpen === open && ((p === null && rect === null) ||
        (p && rect && p.x === rect.x && p.y === rect.y && p.width === rect.width && p.height === rect.height))) return
    protectedRegions = protectedRegions.map(function(r) {
      return r.owner !== owner ? r : {owner:r.owner,output:r.output,edge:r.edge,size:r.size,popupOpen:open,popup:rect}
    })
  }
  function unprotect(owner) {
    protectedRegions = protectedRegions.filter(function(r) { return r.owner !== owner })
  }
  property bool cameraActive: false
  property int cameraEpoch: 0
  property bool enabled: false
  property string targetOutput: ""
  property string cameraEvidence: ""
  property var cameraLimitations: []
  property real widthPercent: 6
  property real brightnessPercent: 70
  property real warmthPercent: 30
  property string pendingLine: ""
  property int pendingBytes: 0
  property bool lockReady: false
  property bool lockUnlocked: false
  property string lastLockEvent: ""
  property bool droppingLine: false
  readonly property bool controlsAllowed: cameraActive && lockReady && lockUnlocked
  function applyCameraState(state) {
    const wasActive = cameraActive
    cameraActive = state === "active"
    if (!cameraActive || !wasActive) {
      cameraEpoch++
      resetSession()
      lockReady = false
      lockUnlocked = false
    }
  }
  function byteLength(text) {
    return encodeURIComponent(text).replace(/%[A-F\d]{2}/g, "x").length
  }
  function consumeStatusLine(raw) {
    try {
      if (byteLength(raw) > 16384) throw new Error("oversized")
      const value = JSON.parse(raw)
      const camera = value.camera
      if (value.version !== 1 || !camera ||
          ["active", "unknown", "unobserved"].indexOf(camera.state) < 0 ||
          typeof camera.evidence !== "string" || !Array.isArray(camera.limitations) ||
          !camera.limitations.every(function(item) { return typeof item === "string" }))
        throw new Error("invalid status")
      cameraEvidence = camera.evidence
      cameraLimitations = camera.limitations.slice()
      applyCameraState(camera.state)
    } catch (error) { clearObserver() }
  }
  function consumeChunk(raw) {
    // SplitParser's empty delimiter forwards chunks without retaining a line.
    for (let i = 0; i < raw.length; i++) {
      const character = raw[i]
      if (character === "\n") {
        const line = pendingLine
        const discard = droppingLine
        pendingLine = ""
        pendingBytes = 0
        droppingLine = false
        if (!discard) consumeStatusLine(line)
      } else if (!droppingLine) {
        pendingLine += character
        const code = character.charCodeAt(0)
        pendingBytes += code < 128 ? 1 : code < 2048 ? 2 : (code >= 0xdc00 && code <= 0xdfff) ? 1 : 3
        if (pendingBytes > 16384) {
          clearObserver()
          droppingLine = true
        }
      }
    }
  }
  function consumeLockStatus(raw, epoch) {
    if (epoch !== undefined && epoch !== cameraEpoch) return
    try {
      if (raw.length > 16384) throw new Error("oversized")
      const value = JSON.parse(raw)
      if (typeof value.locked !== "boolean" || typeof value.sessionLocked !== "boolean" ||
          typeof value.secure !== "boolean" || typeof value.lastEventAt !== "string")
        throw new Error("invalid lock status")
      const changed = lockReady && value.lastEventAt !== lastLockEvent
      lastLockEvent = value.lastEventAt
      lockReady = true
      lockUnlocked = !value.locked && !value.sessionLocked && !value.secure
      if (changed || !lockUnlocked) resetSession()
    } catch (error) { lockFailure() }
  }
  function lockFailure() {
    lockReady = false
    lockUnlocked = false
    resetSession()
  }
  function clearObserver() {
    applyCameraState("unknown")
    cameraEvidence = "Camera observation unavailable"
    cameraLimitations = []
    pendingLine = ""
    pendingBytes = 0
    droppingLine = false
  }
  function enableForOutput(name) {
    if (!controlsAllowed || !name) return
    targetOutput = name
    enabled = true
  }
  function outputRemoved(name) { if (name === targetOutput) disable() }
  function disable() { enabled = false; targetOutput = "" }
  function resetSession() { disable(); sessionReset() }
  function bounded(value, low, high, fallback) {
    return typeof value === "number" && isFinite(value) ? Math.max(low, Math.min(high, value)) : fallback
  }
  function setWidth(value) { widthPercent = bounded(value, 2, 15, 6) }
  function setBrightness(value) { brightnessPercent = bounded(value, 10, 100, 70) }
  function setWarmth(value) { warmthPercent = bounded(value, 0, 100, 30) }
}
