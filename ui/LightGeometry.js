.pragma library
function finite(value) { return typeof value === "number" && isFinite(value) }
function barRect(width, height, edge, size) {
  if (![width,height,size].every(finite) || width <= 0 || height <= 0 || size <= 0) return null
  if (edge === "top") return Qt.rect(0,0,width,Math.min(size,height))
  if (edge === "bottom") return Qt.rect(0,height-Math.min(size,height),width,Math.min(size,height))
  if (edge === "left") return Qt.rect(0,0,Math.min(size,width),height)
  if (edge === "right") return Qt.rect(width-Math.min(size,width),0,Math.min(size,width),height)
  return null
}
function localPoint(x, y, originX, originY) {
  return [x,y,originX,originY].every(finite) ? Qt.point(x-originX,y-originY) : null
}
function paddedRect(rect, padding) {
  if (!rect || ![rect.x,rect.y,rect.width,rect.height,padding].every(finite) ||
      rect.width <= 0 || rect.height <= 0 || padding < 0) return null
  return Qt.rect(rect.x-padding,rect.y-padding,rect.width+padding*2,rect.height+padding*2)
}
