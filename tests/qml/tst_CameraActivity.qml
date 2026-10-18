import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "CameraActivity"
  when: windowShown
  visible: true
  width: 400; height: 300
  CameraActivity { id: activity; width: 300 }
  function row(app, source, evidence) {
    return {application: app, applicationSource: source, device: null,
      devicePath: null, evidence: evidence || "device-access"}
  }
  function init() {
    activity.maximumHeight = 160
    activity.observations = []
    activity.truncated = false
  }
  function label(name) { return findChild(activity, name) }
  function test_unknown_and_provenance() {
    activity.observations = [row(null, "unknown"), row("Browser", "pipewire-reported", "pipewire-capture"),
      row("process", "process")]
    wait(0)
    compare(label("cameraApplication0").text, "Unknown application")
    compare(label("cameraDevice0").text, "Unknown camera")
    compare(label("cameraApplication1").text, "Reported app: Browser")
    compare(label("cameraApplication2").text, "Process: process")
    compare(label("cameraEvidence0").text, "Camera device accessed")
    compare(label("cameraEvidence1").text, "Camera capture observed")
    compare(label("cameraApplication1").Accessible.name, "Reported app: Browser")
  }
  function test_plain_text_and_path() {
    const markup = '<img src="https://invalid/">'
    activity.observations = [{application: markup, applicationSource: "process",
      device: "<b>Webcam</b>", devicePath: "/dev/video0", evidence: "device-access"}]
    wait(0)
    compare(label("cameraApplication0").text, "Process: " + markup)
    compare(label("cameraDevice0").text, "<b>Webcam</b> · /dev/video0")
    ;["cameraApplication0", "cameraDevice0", "cameraEvidence0"].forEach(function(name) {
      compare(label(name).textFormat, Text.PlainText)
    })
  }
  function test_bounded_scrolling_and_resize() {
    activity.observations = Array(32).fill(row("Long application name ".repeat(3), "process"))
    activity.truncated = true
    wait(50)
    verify(activity.contentHeight > 160)
    compare(activity.implicitHeight, 160)
    const scroll = label("cameraActivityFlickable")
    verify(scroll.contentHeight > scroll.height)
    scroll.contentY = scroll.contentHeight - scroll.height
    wait(0)
    verify(scroll.atYEnd)
    compare(label("cameraTruncated").text, "More observations omitted")
    activity.maximumHeight = 70
    wait(0)
    compare(activity.implicitHeight, 70)
    activity.maximumHeight = 500
    compare(activity.implicitHeight, 160)
    activity.observations = []; activity.truncated = false
    wait(0)
    compare(activity.implicitHeight, 0)
    compare(activity.visible, false)
  }
}
