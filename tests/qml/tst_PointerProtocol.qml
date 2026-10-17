import QtQuick
import QtTest
import "../../ui/PointerProtocol.js" as P

TestCase {
  name: "PointerProtocol"
  function test_cadence() {
    compare(P.canRequest(49,0),false)
    compare(P.canRequest(50,0),true)
    compare(P.canRequest(9,10),false)
  }
  function test_fragments() {
    let r = P.begin(100,3)
    r = P.append(r,'{"x":-1920,')
    r = P.append(r,'"y":40.5}')
    compare(P.finish(r,200,3), {valid:true,x:-1920,y:40.5})
    compare(P.finish(r,400,3).valid,false)
    compare(P.finish(r,200,4).valid,false)
  }
  function test_invalid_responses() {
    for (const text of ['{}','{"x":"1","y":2}','oops','{"x":1e400,"y":2}', 'x'.repeat(4097)]) {
      const r = P.append(P.begin(0,1),text)
      compare(P.finish(r,10,1).valid,false)
      verify(r.buffer.length <= 4096)
    }
  }
}
