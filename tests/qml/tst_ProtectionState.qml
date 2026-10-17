import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "ProtectionState"
  EdgeLightState { id: state }
  QtObject { id: first }
  QtObject { id: second }
  function init() { state.protectedRegions = [] }
  function test_owner_and_popup_lifecycle() {
    state.protect(first,"DP-1","top",26)
    state.protect(second,"DP-2","left",28)
    compare(state.protectedRegions.length,2)
    state.protectPopup(first,true,Qt.rect(-1920,26,320,400))
    state.protect(first,"DP-1","bottom",26)
    compare(state.protectedRegions.length,2)
    const r = state.protectedRegions.find(function(r) { return r.owner === first })
    compare(r.popupOpen,true); compare(r.edge,"bottom"); compare(r.popup.x,-1920)
    state.protectPopup(first,false,null)
    compare(state.protectedRegions.find(function(r) { return r.owner === first }).popupOpen,false)
    state.unprotect(first)
    compare(state.protectedRegions.length,1)
    compare(state.protectedRegions[0].output,"DP-2")
  }
}
