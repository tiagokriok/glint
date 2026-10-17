import QtQuick
import QtTest
import "../../ui/LightGeometry.js" as G

TestCase {
  name: "LightGeometry"
  function test_bars() {
    compare(G.barRect(1920,1080,"top",26), Qt.rect(0,0,1920,26))
    compare(G.barRect(1920,1080,"bottom",26), Qt.rect(0,1054,1920,26))
    compare(G.barRect(1080,1920,"left",28), Qt.rect(0,0,28,1920))
    compare(G.barRect(1080,1920,"right",28), Qt.rect(1052,0,28,1920))
    compare(G.barRect(10,10,"top",20), Qt.rect(0,0,10,10))
    compare(G.barRect(0,10,"top",26), null)
    compare(G.barRect(100,100,"unknown",26), null)
  }
  function test_mapping() {
    compare(G.localPoint(-1800,150,-1920,0), Qt.point(120,150))
    compare(G.localPoint(120.5,50.25,100,20), Qt.point(20.5,30.25))
    compare(G.localPoint(NaN,0,0,0), null)
    compare(G.paddedRect(Qt.rect(10,20,100,50),8), Qt.rect(2,12,116,66))
  }
}
