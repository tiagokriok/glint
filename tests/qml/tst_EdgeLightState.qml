import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "EdgeLightState"
  EdgeLightState { id: state }
  SignalSpy { id: resetSpy; target: state; signalName: "sessionReset" }

  function line(status) {
    return JSON.stringify({version: 1, camera: {
      state: status, evidence: "Camera device accessed",
      limitations: ["Direct device access is not confirmed streaming"]
    }})
  }
  function unlocked(stamp) {
    state.consumeLockStatus(JSON.stringify({locked: false, sessionLocked: false,
      secure: false, lastEventAt: stamp === undefined ? "initial" : stamp}))
  }
  function ready() { state.consumeStatusLine(line("active")); unlocked() }
  function init() { state.clearObserver(); state.setWidth(6); state.setBrightness(70); state.setWarmth(30) }

  function test_start_disabled() {
    compare(state.cameraActive, false); compare(state.enabled, false)
    state.enableForOutput("DP-1"); compare(state.enabled, false)
  }
  function test_gate_and_stop() {
    ready(); state.enableForOutput("DP-1")
    compare(state.enabled, true); compare(state.targetOutput, "DP-1")
    state.consumeStatusLine(line("unobserved"))
    compare(state.enabled, false); compare(state.targetOutput, "")
    ready(); compare(state.enabled, false)
  }
  function test_one_output() {
    ready(); state.enableForOutput("DP-1"); state.enableForOutput("DP-2")
    compare(state.targetOutput, "DP-2")
    state.outputRemoved("DP-1"); compare(state.enabled, true)
    state.outputRemoved("DP-2"); compare(state.enabled, false)
    state.enableForOutput(""); compare(state.enabled, false)
  }
  function test_lock_gate_and_reset() {
    state.consumeStatusLine(line("active")); state.enableForOutput("DP-1")
    compare(state.enabled, false)
    unlocked(); state.enableForOutput("DP-1"); compare(state.enabled, true)
    state.consumeLockStatus('{"locked":true,"sessionLocked":true,"secure":true,"lastEventAt":"locked"}')
    compare(state.enabled, false); compare(state.cameraActive, true)
    unlocked("unlock"); compare(state.enabled, false)
    state.enableForOutput("DP-1"); compare(state.enabled, true)
    unlocked("another-event"); compare(state.enabled, false)
  }
  function test_unknown_lock_disables() {
    ready(); state.enableForOutput("DP-1"); state.consumeLockStatus("broken")
    compare(state.enabled, false); compare(state.controlsAllowed, false)
    state.enableForOutput("DP-1"); compare(state.enabled, false)
    unlocked(); compare(state.enabled, false)
  }
  function test_bad_lock_fields_data() {
    return [{tag:"missing", raw:'{}'}, {tag:"wrong-type", raw:'{"locked":"false"}'},
      {tag:"stamp-type", raw:'{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":2}'}]
  }
  function test_bad_lock_fields(data) {
    ready(); state.enableForOutput("DP-1"); state.consumeLockStatus(data.raw)
    compare(state.enabled, false); compare(state.controlsAllowed, false)
  }
  function test_bad_protocol_data() {
    return [{tag:"json", raw:"{"}, {tag:"missing", raw:'{"version":1}'},
      {tag:"version", raw:line("active").replace('"version":1','"version":2')},
      {tag:"state", raw:line("recording")}, {tag:"oversized", raw:"x".repeat(16385)},
      {tag:"limitation-type", raw:'{"version":1,"camera":{"state":"active","evidence":"x","limitations":[3]}}'}]
  }
  function test_bad_protocol(data) {
    ready(); state.enableForOutput("DP-1"); state.consumeStatusLine(data.raw)
    compare(state.cameraActive, false); compare(state.enabled, false)
  }
  function test_access_evidence() {
    ready(); compare(state.cameraEvidence, "Camera device accessed")
    compare(state.cameraLimitations[0], "Direct device access is not confirmed streaming")
  }
  function test_initial_empty_lock_stamp_is_valid() {
    state.consumeStatusLine(line("active")); unlocked("")
    state.enableForOutput("DP-1"); compare(state.enabled, true)
  }
  function test_fast_unlock_event_requests_popup_close() {
    ready(); state.enableForOutput("DP-1"); resetSpy.clear()
    unlocked("new-event")
    compare(resetSpy.count, 1)
    compare(state.enabled, false)
  }
  function test_stale_lock_reply_cannot_unlock_new_camera_session() {
    ready()
    const oldEpoch = state.cameraEpoch
    state.clearObserver(); state.consumeStatusLine(line("active"))
    state.consumeLockStatus('{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":"old"}', oldEpoch)
    compare(state.controlsAllowed, false)
  }
  function test_chunk_framing_bounded() {
    state.consumeChunk(line("active").slice(0, 10)); compare(state.cameraActive, false)
    state.consumeChunk(line("active").slice(10) + "\n"); compare(state.cameraActive, true)
    state.consumeChunk("x".repeat(17000)); compare(state.cameraActive, false)
    compare(state.pendingLine.length, 0)
    state.consumeChunk(line("active") + "\n"); compare(state.cameraActive, false)
    state.consumeChunk(line("active") + "\n"); compare(state.cameraActive, true)
  }
  function test_unterminated_unicode_line_is_byte_bounded() {
    state.consumeChunk("é".repeat(9000))
    compare(state.pendingLine.length, 0)
    compare(state.droppingLine, true)
  }
  function test_reset_and_ranges() {
    ready(); state.enableForOutput("DP-1"); state.resetSession()
    compare(state.enabled, false); compare(state.cameraActive, true)
    state.setWidth(100); compare(state.widthPercent, 15)
    state.setBrightness(-1); compare(state.brightnessPercent, 10)
    state.setWarmth(NaN); compare(state.warmthPercent, 30)
    state.setWidth(Infinity); compare(state.widthPercent, 6)
  }
}
