import QtQuick
import QtQuick.Effects

Item {
  id: root
  property real widthPercent: 6
  property real brightnessPercent: 70
  property real warmthPercent: 30
  property var protectedRects: []
  property point pointer: Qt.point(-1000,-1000)
  property bool pointerValid: false
  EdgeLightBorder {
    id: sourceLight
    anchors.fill: parent
    visible: false
    layer.enabled: true
    widthPercent: root.widthPercent
    brightnessPercent: 100
    warmthPercent: root.warmthPercent
  }
  LightMask {
    id: clearRegions
    anchors.fill: parent
    visible: false
    protectedRects: root.protectedRects
    pointer: root.pointer
    pointerValid: root.pointerValid
  }
  MultiEffect {
    anchors.fill: parent
    source: sourceLight
    opacity: root.brightnessPercent / 100
    autoPaddingEnabled: false
    maskEnabled: true
    maskInverted: true
    maskThresholdMin: 0.5
    maskSpreadAtMin: 1
    maskSource: clearRegions
  }
}
