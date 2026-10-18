pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls

Control {
  id: root
  property var observations: []
  property bool truncated: false
  property real maximumHeight: 160
  readonly property real contentHeight: visible ? rows.implicitHeight : 0
  visible: observations.length > 0 || truncated
  implicitHeight: Math.min(contentHeight, Math.max(0, Math.min(160, maximumHeight)))
  padding: 0
  contentItem: ScrollView {
    id: scroll
    contentWidth: availableWidth
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    Flickable {
      objectName: "cameraActivityFlickable"
      clip: true
      contentWidth: width
      contentHeight: rows.implicitHeight
      boundsBehavior: Flickable.StopAtBounds
      Column {
        id: rows
        width: parent.width - scroll.effectiveScrollBarWidth
        spacing: 10
        Label {
          text: "Camera activity"
          font.bold: true
          textFormat: Text.PlainText
        }
        Repeater {
          model: root.observations
          delegate: Column {
            id: observation
            required property var modelData
            required property int index
            width: rows.width
            spacing: 2
            Label {
              objectName: "cameraApplication" + observation.index
              width: parent.width
              text: !observation.modelData.application || observation.modelData.applicationSource === "unknown"
                ? "Unknown application"
                : (observation.modelData.applicationSource === "process" ? "Process: " : "Reported app: ")
                  + observation.modelData.application
              textFormat: Text.PlainText
              wrapMode: Text.Wrap
              Accessible.name: text
            }
            Label {
              objectName: "cameraDevice" + observation.index
              width: parent.width
              text: (observation.modelData.device || "Unknown camera")
                + (observation.modelData.devicePath ? " · " + observation.modelData.devicePath : "")
              textFormat: Text.PlainText
              wrapMode: Text.Wrap
              Accessible.name: text
            }
            Label {
              objectName: "cameraEvidence" + observation.index
              width: parent.width
              text: observation.modelData.evidence === "pipewire-capture"
                ? "Camera capture observed" : "Camera device accessed"
              textFormat: Text.PlainText
              wrapMode: Text.Wrap
              Accessible.name: text
            }
          }
        }
        Label {
          objectName: "cameraTruncated"
          width: parent.width
          visible: root.truncated
          text: "More observations omitted"
          textFormat: Text.PlainText
          wrapMode: Text.Wrap
          Accessible.name: text
        }
      }
    }
  }
}
