import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "ProtectedLight"
  when: windowShown
  visible: true
  width: 400; height: 300
  Rectangle { anchors.fill: parent; color: "black" }
  CameraActivity { id: activity; x: 500; width: 180 }
  ProtectedLight {
    id: light
    anchors.fill: parent
    widthPercent: 15; brightnessPercent: 100; warmthPercent: 0
    protectedRects: [Qt.rect(0,0,400,26),Qt.rect(0,100,180,140)]
    pointer: Qt.point(400,150)
    pointerValid: true
  }
  function init() {
    if (light.GraphicsInfo.api === GraphicsInfo.Software) skip("GPU mask requires QT_QUICK_BACKEND=rhi")
    light.brightnessPercent = 100
    light.pointer = Qt.point(400,150)
    light.protectedRects = [Qt.rect(0,0,400,26),Qt.rect(0,100,180,140)]
    activity.observations = []
  }
  function test_clear_regions_and_pointer() {
    wait(100)
    const image = grabImage(light)
    verify(image.red(200,10) < 10, "bar must reveal the black background")
    verify(image.red(10,150) < 10, "popup must reveal the black background")
    verify(image.red(395,150) < 10, "pointer hole must reveal the black background")
    verify(image.red(200,295) > 240, "distant edge must stay lit")
  }
  function test_feather_and_brightness_once() {
    light.pointer = Qt.point(400,150)
    light.brightnessPercent = 70
    wait(100)
    const image = grabImage(light)
    verify(image.red(200,295) >= 175 && image.red(200,295) <= 181)
    verify(image.red(395,45) > 5 && image.red(395,45) < 175, "feather pixel: " + image.red(395,45))
  }
  function test_overlapping_holes_never_relight() {
    light.pointer = Qt.point(0,150)
    wait(100)
    const image = grabImage(light)
    verify(image.red(10,150) < 10)
    verify(image.red(395,150) > 240)
  }
  function test_activity_growth_preserves_popup_bar_and_pointer_holes() {
    light.protectedRects = Qt.binding(function() {
      return [Qt.rect(0,0,400,26), Qt.rect(0,100,180,activity.implicitHeight + 20)]
    })
    wait(100)
    verify(grabImage(light).red(10,200) > 240, "edge below a small popup starts lit")
    activity.observations = Array(12).fill({application: "Observed process", applicationSource: "process",
      device: null, devicePath: null, evidence: "device-access"})
    wait(100)
    compare(activity.implicitHeight, 160)
    const image = grabImage(light)
    verify(image.red(10,200) < 10, "growing activity must extend the popup cutout")
    verify(image.red(200,10) < 10, "bar stays clear while popup grows")
    verify(image.red(395,150) < 10, "pointer recession remains independent of popup growth")
    verify(image.red(200,295) > 240, "unprotected edge stays lit")
    activity.observations = []
    wait(100)
    verify(grabImage(light).red(10,200) > 240, "shrinking popup releases its old cutout")
  }
}
