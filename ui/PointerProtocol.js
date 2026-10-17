.pragma library
function canRequest(now, last) { return now-last >= 50 }
function begin(now, epoch) { return {buffer:"",started:now,epoch:epoch,failed:false} }
function append(request, chunk) {
  if (!request || request.failed) return request
  const buffer = request.buffer + chunk
  let failed = true
  try { failed = encodeURIComponent(buffer).replace(/%[A-F\d]{2}/g,"x").length > 4096 } catch(error) {}
  return {buffer:failed ? "" : buffer,started:request.started,epoch:request.epoch,failed:failed}
}
function finish(request, now, epoch) {
  if (!request || request.failed || request.epoch !== epoch || now-request.started > 250 || now < request.started) return {valid:false}
  try {
    const p = JSON.parse(request.buffer)
    if (typeof p.x === "number" && isFinite(p.x) && typeof p.y === "number" && isFinite(p.y)) return {valid:true,x:p.x,y:p.y}
  } catch(error) {}
  return {valid:false}
}
