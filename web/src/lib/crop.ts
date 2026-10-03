// The arithmetic of the crop frame: a rectangle on the picture, in the
// picture's own pixels, that can be moved and resized by its handles, with an
// optional fixed aspect ratio. All of it works in whole pixels and never lets
// the frame leave the picture.

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Size {
  width: number;
  height: number;
}

/** Which part of the frame is being dragged. */
export type Handle = 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

export const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

/** The smallest frame edge that can be dragged to, in pixels. */
export const MIN_EDGE = 4;

/** The aspect ratios offered; `null` is a free frame. */
export const ASPECTS = ['free', '1:1', '4:3', '3:2', '16:9'] as const;
export type AspectChoice = (typeof ASPECTS)[number];

/** "16:9" as a number (width / height); `null` for "free". */
export function aspectValue(choice: AspectChoice): number | null {
  if (choice === 'free') return null;
  const [w, h] = choice.split(':').map(Number);
  return w / h;
}

export function fullRect(bounds: Size): Rect {
  return { x: 0, y: 0, width: bounds.width, height: bounds.height };
}

/** The frame, in whole pixels, inside `bounds`, at least one pixel big. */
export function clampRect(rect: Rect, bounds: Size): Rect {
  const width = Math.min(Math.max(Math.round(rect.width), 1), bounds.width);
  const height = Math.min(Math.max(Math.round(rect.height), 1), bounds.height);
  const x = Math.min(Math.max(Math.round(rect.x), 0), bounds.width - width);
  const y = Math.min(Math.max(Math.round(rect.y), 0), bounds.height - height);
  return { x, y, width, height };
}

/** Whether the frame is the whole picture, which needs no crop at all. */
export function isFull(rect: Rect, bounds: Size): boolean {
  return rect.x === 0 && rect.y === 0 && rect.width === bounds.width && rect.height === bounds.height;
}

/**
 * The largest frame of the aspect ratio that fits in `rect`, with the same
 * centre - what the frame becomes when a ratio is chosen.
 */
export function fitAspect(rect: Rect, aspect: number, bounds: Size): Rect {
  let width = rect.width;
  let height = width / aspect;
  if (height > rect.height) {
    height = rect.height;
    width = height * aspect;
  }
  const centreX = rect.x + rect.width / 2;
  const centreY = rect.y + rect.height / 2;
  return clampRect({ x: centreX - width / 2, y: centreY - height / 2, width, height }, bounds);
}

/**
 * The frame after dragging `handle` by (`dx`, `dy`) pixels from where it was
 * when the drag began. With an `aspect`, the frame keeps that ratio.
 */
export function dragRect(
  start: Rect,
  handle: Handle,
  dx: number,
  dy: number,
  bounds: Size,
  aspect: number | null = null,
): Rect {
  if (handle === 'move') {
    return clampRect({ ...start, x: start.x + dx, y: start.y + dy }, bounds);
  }
  const west = handle.includes('w');
  const east = handle.includes('e');
  const north = handle.includes('n');
  const south = handle.includes('s');
  const corner = (west || east) && (north || south);

  if (aspect === null) {
    let left = start.x;
    let right = start.x + start.width;
    let top = start.y;
    let bottom = start.y + start.height;
    if (west) left = Math.min(Math.max(left + dx, 0), right - MIN_EDGE);
    if (east) right = Math.max(Math.min(right + dx, bounds.width), left + MIN_EDGE);
    if (north) top = Math.min(Math.max(top + dy, 0), bottom - MIN_EDGE);
    if (south) bottom = Math.max(Math.min(bottom + dy, bounds.height), top + MIN_EDGE);
    return clampRect({ x: left, y: top, width: right - left, height: bottom - top }, bounds);
  }

  // With a fixed ratio the edge or corner opposite to the handle stays where
  // it is, and the frame grows or shrinks from it.
  const anchorX = west ? start.x + start.width : east ? start.x : start.x + start.width / 2;
  const anchorY = north ? start.y + start.height : south ? start.y : start.y + start.height / 2;
  const roomX = west ? anchorX : east ? bounds.width - anchorX : 2 * Math.min(anchorX, bounds.width - anchorX);
  const roomY = north ? anchorY : south ? bounds.height - anchorY : 2 * Math.min(anchorY, bounds.height - anchorY);
  const movedX = (west ? -1 : 1) * (corner || west || east ? dx : 0);
  const movedY = (north ? -1 : 1) * (corner || north || south ? dy : 0);

  let width: number;
  if (corner) {
    // Follow whichever of the two directions the pointer went further in.
    const byX = start.width + movedX;
    const byY = (start.height + movedY) * aspect;
    width = Math.max(byX, byY);
  } else if (west || east) {
    width = start.width + movedX;
  } else {
    width = (start.height + movedY) * aspect;
  }
  width = Math.min(Math.max(width, MIN_EDGE * Math.max(aspect, 1)), roomX, roomY * aspect);
  const height = width / aspect;

  const x = west ? anchorX - width : east ? anchorX : anchorX - width / 2;
  const y = north ? anchorY - height : south ? anchorY : anchorY - height / 2;
  return clampRect({ x, y, width, height }, bounds);
}

/**
 * The frame after zooming by `factor` (below 1 zooms in: the frame covers less
 * of the picture). The point of the frame at (`ax`, `ay`), as fractions of its
 * width and height, stays where it is on the screen - the point under the mouse.
 * The shape of the frame stays the same, and it never grows past the picture.
 */
export function zoomRect(rect: Rect, factor: number, ax: number, ay: number, bounds: Size): Rect {
  const ratio = rect.width / rect.height;
  const biggest = Math.min(bounds.width, bounds.height * ratio);
  const smallest = Math.max(MIN_EDGE, MIN_EDGE * ratio);
  const width = Math.min(Math.max(rect.width * factor, smallest), biggest);
  const height = width / ratio;
  const x = rect.x + ax * (rect.width - width);
  const y = rect.y + ay * (rect.height - height);
  return clampRect({ x, y, width, height }, bounds);
}

/** The frame moved by (`dx`, `dy`) picture pixels, kept inside the picture. */
/**
 * The largest frame of the same shape the picture allows (the shape of `aspect`, or of the frame
 * itself where that is free), around the frame's own middle as far as the picture lets it.
 */
export function growRect(rect: Rect, aspect: number | null, bounds: Size): Rect {
  const ratio = aspect ?? rect.width / rect.height;
  let width = bounds.width;
  let height = width / ratio;
  if (height > bounds.height) {
    height = bounds.height;
    width = height * ratio;
  }
  width = Math.max(1, Math.round(width));
  height = Math.max(1, Math.round(height));
  const x = rect.x + rect.width / 2 - width / 2;
  const y = rect.y + rect.height / 2 - height / 2;
  return clampRect({ x, y, width, height }, bounds);
}

/**
 * The frame turned by a quarter: wide becomes tall, around its middle. Where the turned frame is
 * too big for the picture it is made smaller, keeping its new shape.
 */
export function rotateRect(rect: Rect, bounds: Size): Rect {
  let width = rect.height;
  let height = rect.width;
  const shrink = Math.min(1, bounds.width / width, bounds.height / height);
  width = Math.max(1, Math.round(width * shrink));
  height = Math.max(1, Math.round(height * shrink));
  const x = rect.x + rect.width / 2 - width / 2;
  const y = rect.y + rect.height / 2 - height / 2;
  return clampRect({ x, y, width, height }, bounds);
}

/** One of three places along an axis: the start (left, top), the middle or the end (right, bottom). */
export type Place = 0 | 1 | 2;

/** The frame moved, without changing its size, to the left/middle/right (`h`) and top/middle/bottom (`v`) of the picture. */
export function alignRect(rect: Rect, bounds: Size, h: Place, v: Place): Rect {
  const along = (place: Place, free: number) => (place === 0 ? 0 : place === 1 ? Math.round(free / 2) : free);
  return clampRect(
    { ...rect, x: along(h, bounds.width - rect.width), y: along(v, bounds.height - rect.height) },
    bounds,
  );
}

export function panRect(rect: Rect, dx: number, dy: number, bounds: Size): Rect {
  return clampRect({ ...rect, x: rect.x + dx, y: rect.y + dy }, bounds);
}
