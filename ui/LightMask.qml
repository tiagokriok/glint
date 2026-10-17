pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Shapes

Item {
  id: root
  property var protectedRects: []
  property point pointer: Qt.point(-1000,-1000)
  property bool pointerValid: false
  layer.enabled: true
  Repeater {
    model: root.protectedRects
    delegate: Rectangle {
      required property var modelData
      x: modelData.x; y: modelData.y
      width: modelData.width; height: modelData.height
      color: "white"
    }
  }
  Shape {
    visible: root.pointerValid
    x: root.pointer.x - 128; y: root.pointer.y - 128
    width: 256; height: 256
    ShapePath {
      strokeColor: "transparent"
      fillGradient: RadialGradient {
        centerX: 128; centerY: 128; centerRadius: 128
        focalX: 128; focalY: 128
        GradientStop { position: 0; color: "white" }
        GradientStop { position: 0.75; color: "white" }
        GradientStop { position: 1; color: "transparent" }
      }
      PathAngleArc { centerX: 128; centerY: 128; radiusX: 128; radiusY: 128; startAngle: 0; sweepAngle: 360 }
    }
  }
}
