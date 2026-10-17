import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "EdgeLightBorder"
  EdgeLightBorder { id: border; width: 1920; height: 1080 }
  function test_geometry() {
    compare(border.thickness, 64.8)
    border.width = 1080; border.height = 1920
    compare(border.thickness, 64.8)
    border.widthPercent = 15; compare(border.thickness, 162)
    border.widthPercent = 2; compare(border.thickness, 21.6)
    const strips = border.children
    compare(strips.length, 4)
    compare(strips[0].height, border.thickness)
    compare(strips[2].y, border.thickness)
    compare(strips[2].height, border.height - 2 * border.thickness)
  }
  function test_light_values() {
    border.warmthPercent = 0; compare(border.lightColor, Qt.rgba(1,1,1,1))
    border.warmthPercent = 100; compare(border.lightColor, Qt.rgba(1,214/255,170/255,1))
    border.brightnessPercent = 70; compare(border.opacity, 0.7)
  }
}
