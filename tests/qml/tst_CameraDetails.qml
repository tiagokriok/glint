import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "CameraDetails"
  EdgeLightState { id: state }
  function row(overrides) {
    return Object.assign({application: null, applicationSource: "unknown", device: null,
      devicePath: null, evidence: "device-access"}, overrides || {})
  }
  function details(rows) { return {observations: rows || [row()], truncated: false} }
  function send(value, status) {
    state.consumeStatusLine(JSON.stringify({version: 1, camera: {
      state: status || "active", evidence: "fixture", limitations: [], details: value}}))
  }
  function init() { state.clearObserver() }
  function test_old_helper() {
    send(details()); send(undefined)
    compare(state.cameraActive, true)
    compare(state.cameraObservations.length, 0)
    compare(state.cameraDetailsTruncated, false)
  }
  function test_supported_rows() {
    const rows = [row(), row({application: "Browser", applicationSource: "pipewire-reported",
      device: "Webcam", devicePath: "/dev/video12", evidence: "pipewire-capture"}),
      row({application: "process", applicationSource: "process"})]
    send({observations: rows, truncated: true})
    compare(state.cameraObservations, rows)
    compare(state.cameraDetailsTruncated, true)
  }
  function test_invalid_data() {
    const blocks = [null, [], {}, {observations: [], truncated: 1},
      {observations: {}, truncated: false}, details([null]), details([[]]),
      details([{}]), details(Array(33).fill(row()))]
    const invalid = {application: [undefined, 3, {}, [], "x".repeat(81), "é".repeat(41), "bad\u0000", "bad\u0085",
        "bad\u061c", "bad\u200e", "bad\u202e", "bad\u2067", "\ud800"],
      applicationSource: [undefined, null, "browser", 1], device: [undefined, false, "x".repeat(81), "bad\n"],
      devicePath: [undefined, 1, "/dev/video", "/dev/video1/../video2", "file:///dev/video0",
        "/dev/video0\n", "/dev/video" + "1".repeat(60)],
      evidence: [undefined, null, "active", 1]}
    const rowBlocks = Object.keys(invalid).reduce(function(all, key) {
      return all.concat(invalid[key].map(function(value) {
        return details([row(), row({[key]: value})])
      }))
    }, [])
    return blocks.concat(rowBlocks).map(function(value, i) { return {tag: "invalid-" + i, value: value} })
  }
  function test_invalid(data) {
    send({observations: [row()], truncated: true})
    send(data.value)
    compare(state.cameraActive, true, "optional corruption must not invalidate detection")
    compare(state.cameraObservations.length, 0, "reject the entire optional block")
    compare(state.cameraDetailsTruncated, false)
  }
  function test_boundaries_and_literal_markup() {
    const rows = Array(32).fill(row({application: "é".repeat(40),
      device: '<img src="https://invalid/">', devicePath: "/dev/video" + "1".repeat(54)}))
    send(details(rows)); compare(state.cameraObservations.length, 32)
    compare(state.cameraObservations[0].device, rows[0].device)
  }
  function test_metadata_only_update_preserves_light() {
    send(details())
    state.consumeLockStatus('{"locked":false,"sessionLocked":false,"secure":false,"lastEventAt":"ready"}')
    state.enableForOutput("DP-1")
    state.setWidth(11); state.setBrightness(83); state.setWarmth(54)
    const epoch = state.cameraEpoch
    const updates = [details([row({application: "new", applicationSource: "process"})]), null, undefined]
    updates.forEach(function(value) {
      send(value)
      compare(state.enabled, true); compare(state.targetOutput, "DP-1")
      compare(state.cameraEpoch, epoch); compare(state.lockReady, true)
      compare(state.lockUnlocked, true); compare(state.controlsAllowed, true)
      compare(state.lastLockEvent, "ready")
      compare(state.widthPercent, 11); compare(state.brightnessPercent, 83); compare(state.warmthPercent, 54)
    })
  }
  function test_lifecycle() {
    ["unobserved", "unknown"].forEach(function(status) {
      send(details()); send(details(), status)
      compare(state.cameraObservations.length, 0)
      send(undefined); compare(state.cameraObservations.length, 0)
    })
    send(details()); state.clearObserver()
    compare(state.cameraObservations.length, 0)
    send(undefined); compare(state.cameraObservations.length, 0)
    send(details()); state.resetSession()
    compare(state.cameraObservations.length, 0)
    compare(state.cameraDetailsTruncated, false)
  }
}
