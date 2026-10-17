import QtQuick

Item {
  id: root
  property real widthPercent: 6
  property real brightnessPercent: 70
  property real warmthPercent: 30
  readonly property real thickness: Math.min(width, height) * widthPercent / 100
  readonly property color lightColor: Qt.rgba(1, 1 - (41 / 255) * warmthPercent / 100,
    1 - (85 / 255) * warmthPercent / 100, 1)
  opacity: brightnessPercent / 100

  Rectangle { x: 0; y: 0; width: root.width; height: root.thickness; color: root.lightColor }
  Rectangle { x: 0; y: root.height - root.thickness; width: root.width; height: root.thickness; color: root.lightColor }
  Rectangle { x: 0; y: root.thickness; width: root.thickness; height: root.height - 2 * root.thickness; color: root.lightColor }
  Rectangle { x: root.width - root.thickness; y: root.thickness; width: root.thickness; height: root.height - 2 * root.thickness; color: root.lightColor }
}
