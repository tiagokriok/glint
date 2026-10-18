pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import qs.Ui
import "LightGeometry.js" as Geometry

PopupCard {
  id: root
  objectName: "edgeLightPopup"
  required property var lightState
  required property string outputName
  signal protectedRegion(bool open, var rect)
  function reportGeometry() {
    let rect = null
    if (open && anchorWindow && popupScreen) {
      try {
        const point = content.mapToGlobal(0, 0)
        rect = Geometry.paddedRect(Qt.rect(point.x, point.y, content.width, content.height), padding + 8)
      } catch (error) {}
    }
    protectedRegion(open, rect)
  }
  onOpenChanged: Qt.callLater(reportGeometry)
  onContentWidthChanged: Qt.callLater(reportGeometry)
  onContentHeightChanged: Qt.callLater(reportGeometry)
  contentWidth: fittedContentWidth(360)
  contentHeight: fittedContentHeight(heading.implicitHeight + toggle.implicitHeight
    + Math.min(160, activity.contentHeight) + controls.implicitHeight + content.spacing * 3, 560)

  ColumnLayout {
    id: content
    focus: true
    Keys.onEscapePressed: root.close()
    anchors.fill: parent
    spacing: 8
    onWidthChanged: Qt.callLater(root.reportGeometry)
    onHeightChanged: Qt.callLater(root.reportGeometry)
    Timer { interval: 50; repeat: true; running: root.open; onTriggered: root.reportGeometry() }
    CameraActivity {
      id: activity
      objectName: "cameraActivity"
      Layout.fillWidth: true
      Layout.preferredHeight: implicitHeight
      maximumHeight: Math.max(0, content.height - heading.implicitHeight - toggle.implicitHeight
        - content.spacing * 3 - 60)
      observations: root.lightState ? root.lightState.cameraObservations : []
      truncated: root.lightState ? root.lightState.cameraDetailsTruncated : false
      palette.windowText: Color.popups.text
      onContentHeightChanged: Qt.callLater(root.reportGeometry)
      onHeightChanged: Qt.callLater(root.reportGeometry)
    }
    Label {
      id: heading
      text: "Edge Light"
      color: Color.popups.text
      font.bold: true
      font.pixelSize: 18
    }
    Switch {
      id: toggle
      objectName: "edgeLightToggle"
      text: "Illuminate this display"
      checked: root.lightState ? root.lightState.enabled : false
      enabled: root.lightState ? root.lightState.controlsAllowed : false
      palette.windowText: Color.popups.text
      palette.highlight: Color.bar.active
      Accessible.name: text
      onToggled: {
        if (!root.lightState) return
        if (checked) root.lightState.enableForOutput(root.outputName)
        else root.lightState.disable()
      }
    }
    ScrollView {
      id: controlsScroll
      objectName: "edgeLightControlsScroll"
      Layout.fillWidth: true
      Layout.fillHeight: true
      Layout.minimumHeight: 1
      contentWidth: availableWidth
      contentHeight: controls.implicitHeight
      ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
      onHeightChanged: Qt.callLater(root.reportGeometry)
      Flickable {
        clip: true
        contentWidth: width
        contentHeight: controls.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        onContentYChanged: Qt.callLater(root.reportGeometry)
        ColumnLayout {
          id: controls
          width: parent.width - controlsScroll.effectiveScrollBarWidth
          spacing: 10
          onImplicitHeightChanged: Qt.callLater(root.reportGeometry)
          Label {
            Layout.fillWidth: true
            text: root.lightState ? root.lightState.cameraEvidence : ""
            textFormat: Text.PlainText
            color: Color.popups.text
            wrapMode: Text.WordWrap
          }
          Repeater {
            model: [
              {label: "Width", low: 2, high: 15, key: "widthPercent"},
              {label: "Brightness", low: 10, high: 100, key: "brightnessPercent"},
              {label: "Warmth", low: 0, high: 100, key: "warmthPercent"}
            ]
            delegate: ColumnLayout {
              id: row
              required property var modelData
              Layout.fillWidth: true
              spacing: 0
              Label {
                text: row.modelData.label + "  " + Math.round(control.value) + "%"
                color: Color.popups.text
              }
              Slider {
                id: control
                Layout.fillWidth: true
                from: row.modelData.low
                to: row.modelData.high
                stepSize: 1
                value: root.lightState ? root.lightState[row.modelData.key] : from
                palette.highlight: Color.bar.active
                Accessible.name: row.modelData.label
                onMoved: {
                  if (!root.lightState) return
                  if (row.modelData.key === "widthPercent") root.lightState.setWidth(value)
                  else if (row.modelData.key === "brightnessPercent") root.lightState.setBrightness(value)
                  else root.lightState.setWarmth(value)
                }
              }
            }
          }
          Label {
            Layout.fillWidth: true
            text: root.lightState ? root.lightState.cameraLimitations.join(" · ") : ""
            textFormat: Text.PlainText
            color: Color.popups.text
            font.pixelSize: 11
            wrapMode: Text.WordWrap
          }
        }
      }
    }
  }
}
