import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "ProtectedLight"
  when: windowShown
  visible: true
  width: 400; height: 300
  Rectangle { anchors.fill: parent; color: "black" }
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
}
