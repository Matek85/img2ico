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
